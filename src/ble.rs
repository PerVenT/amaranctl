use std::sync::Arc;
use std::time::Duration;
use anyhow::{bail, Context, Result};
use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::mesh::{
    build_proxy_config_pdu, build_proxy_pdu, decode_telink_status, decrypt_proxy_pdu,
    encode_brightness, encode_brightness_raw, encode_cct, encode_onoff, encode_status_request,
    MeshSecurityContext, PROXY_CFG_ADD_ADDRESSES, PROXY_CFG_SET_FILTER_TYPE, PROXY_FILTER_WHITELIST,
};

pub const PROXY_DATA_IN_UUID: Uuid = Uuid::from_u128(0x00002add_0000_1000_8000_00805f9b34fb);
pub const PROXY_DATA_OUT_UUID: Uuid = Uuid::from_u128(0x00002ade_0000_1000_8000_00805f9b34fb);

pub const LOCAL_SRC_ADDR: u16 = 0x0001;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LampStatus {
    pub connected: bool,
    pub connecting: bool,
    pub power: bool,
    pub brightness: f32,
    pub cct: u16,
}

impl Default for LampStatus {
    fn default() -> Self {
        Self {
            connected: false,
            connecting: false,
            power: false,
            brightness: 100.0,
            cct: 5600,
        }
    }
}

impl LampStatus {
    pub fn tooltip_text(&self) -> String {
        if self.connecting {
            "Amaran 60x S — ⏳ Пошук та підключення...".to_string()
        } else if !self.connected {
            "Amaran 60x S — 🔴 Немає зв'язку (перевірте живлення лампи)".to_string()
        } else if self.power {
            format!("Amaran 60x S — 🟢 ON | {:.0}% | {}K", self.brightness, self.cct)
        } else {
            "Amaran 60x S — ⚪ OFF (Готовий)".to_string()
        }
    }
}

pub struct BleMeshClient {
    pub config: AppConfig,
    pub security: MeshSecurityContext,
    pub seq: u32,
    pub iv_index: u32,
    peripheral: Option<Peripheral>,
    char_in: Option<Characteristic>,
    pub last_power: bool,
    pub last_brightness: u16,
    pub last_cct: u16,
    pub status: Arc<RwLock<LampStatus>>,
}

impl BleMeshClient {
    pub fn new(config: AppConfig) -> Result<Self> {
        let sec = MeshSecurityContext::new(&config.net_key_bytes(), &config.app_key_bytes())
            .context("failed to derive mesh security keys")?;

        // Initialize monotonic sequence number in the 12M..16M range
        // to avoid replay protection rejections across program restarts
        let initial_seq = 12_000_000 + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32 % 4_000_000);

        Ok(Self {
            config,
            security: sec,
            seq: initial_seq,
            iv_index: 0,
            peripheral: None,
            char_in: None,
            last_power: true,
            last_brightness: 500, // 50% default
            last_cct: 5600,       // 5600K default daylight
            status: Arc::new(RwLock::new(LampStatus::default())),
        })
    }

    pub async fn current_status(&self) -> LampStatus {
        self.status.read().await.clone()
    }

    pub fn next_seq(&mut self) -> u32 {
        let cur = self.seq;
        self.seq = self.seq.wrapping_add(1);
        cur
    }

    pub fn build_onoff_pdu(&mut self, target_addr: u16, on: bool) -> Result<Vec<u8>> {
        let payload = encode_onoff(on);
        let seq = self.next_seq();
        self.last_power = on;
        build_proxy_pdu(
            &self.security,
            seq,
            LOCAL_SRC_ADDR,
            target_addr,
            self.iv_index,
            0x26,
            &payload,
        )
    }

    pub fn build_brightness_pdu(&mut self, target_addr: u16, percent: f32) -> Result<Vec<u8>> {
        let payload = encode_brightness(percent);
        let seq = self.next_seq();
        self.last_brightness = (percent.clamp(0.0, 100.0) * 10.0).round() as u16;
        build_proxy_pdu(
            &self.security,
            seq,
            LOCAL_SRC_ADDR,
            target_addr,
            self.iv_index,
            0x26,
            &payload,
        )
    }

    pub fn build_brightness_raw_pdu(&mut self, target_addr: u16, raw: u16) -> Result<Vec<u8>> {
        let payload = encode_brightness_raw(raw);
        let seq = self.next_seq();
        self.last_brightness = raw.min(1000);
        build_proxy_pdu(
            &self.security,
            seq,
            LOCAL_SRC_ADDR,
            target_addr,
            self.iv_index,
            0x26,
            &payload,
        )
    }

    pub fn build_cct_pdu(
        &mut self,
        target_addr: u16,
        kelvin: u16,
        intensity: u16,
        gm_offset: i8,
    ) -> Result<Vec<u8>> {
        let payload = encode_cct(kelvin, intensity, gm_offset);
        let seq = self.next_seq();
        self.last_cct = kelvin;
        self.last_brightness = intensity.min(1000);
        build_proxy_pdu(
            &self.security,
            seq,
            LOCAL_SRC_ADDR,
            target_addr,
            self.iv_index,
            0x26,
            &payload,
        )
    }

    pub fn build_status_request_pdu(&mut self, target_addr: u16) -> Result<Vec<u8>> {
        let payload = encode_status_request();
        let seq = self.next_seq();
        build_proxy_pdu(
            &self.security,
            seq,
            LOCAL_SRC_ADDR,
            target_addr,
            self.iv_index,
            0x26,
            &payload,
        )
    }

    pub async fn is_connected(&self) -> bool {
        if let Some(ref p) = self.peripheral {
            p.is_connected().await.unwrap_or(false) && self.char_in.is_some()
        } else {
            false
        }
    }

    pub fn has_peripheral(&self) -> bool {
        self.peripheral.is_some()
    }

    pub async fn ensure_connected(&mut self) -> Result<()> {
        if self.is_connected().await {
            return Ok(());
        }
        self.connect().await
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.status.write().await.connecting = true;
        let manager = Manager::new().await.context("failed to get BLE manager")?;
        let adapters = manager.adapters().await.context("failed to list BLE adapters")?;
        let adapter = adapters.into_iter().next().context("no Bluetooth adapter found on system")?;

        adapter.start_scan(ScanFilter::default()).await.context("failed to start BLE scan")?;

        let light = self.config.primary_light().context("no light configured")?;
        let target_mac = light.mac.replace(':', "").to_uppercase();
        let target_name = &light.name;

        let mut matched_peripheral: Option<Peripheral> = None;
        let scan_start = std::time::Instant::now();

        while scan_start.elapsed() < Duration::from_secs(10) {
            let peripherals = adapter.peripherals().await.unwrap_or_default();
            for p in peripherals {
                let p_addr = p.address().to_string().replace(':', "").to_uppercase();
                if p_addr == target_mac {
                    matched_peripheral = Some(p);
                    break;
                }

                if let Ok(Some(props)) = p.properties().await {
                    let addr = props.address.to_string().replace(':', "").to_uppercase();
                    let name = props.local_name.unwrap_or_default();
                    if addr == target_mac || name == *target_name || name.contains(&target_mac) || name == "Mesh Device" {
                        matched_peripheral = Some(p);
                        break;
                    }
                }
            }
            if matched_peripheral.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        let peripheral = match matched_peripheral {
            Some(p) => p,
            None => {
                let _ = adapter.stop_scan().await;
                let mut s = self.status.write().await;
                s.connecting = false;
                s.connected = false;
                bail!("Could not find target Amaran light '{}' ({})", target_name, target_mac);
            }
        };

        // Note: Connect and discover services BEFORE stopping scan on Windows
        // to prevent WinRT from dropping the peripheral watcher reference
        if let Err(e) = peripheral.connect().await {
            let mut s = self.status.write().await;
            s.connecting = false;
            s.connected = false;
            return Err(e).context("failed to connect to peripheral");
        }
        if let Err(e) = peripheral.discover_services().await {
            let mut s = self.status.write().await;
            s.connecting = false;
            s.connected = false;
            return Err(e).context("failed to discover services");
        }
        let _ = adapter.stop_scan().await;

        let chars = peripheral.characteristics();
        let char_in = chars
            .iter()
            .find(|c| c.uuid == PROXY_DATA_IN_UUID)
            .cloned()
            .context("Proxy Data In characteristic (2ADD) not found")?;

        if let Some(char_out) = chars.iter().find(|c| c.uuid == PROXY_DATA_OUT_UUID) {
            let _ = peripheral.subscribe(char_out).await;
            if let Ok(mut stream) = peripheral.notifications().await {
                let status_clone = self.status.clone();
                let sec_clone = self.security.clone();
                let iv = self.iv_index;
                tokio::spawn(async move {
                    while let Some(notif) = stream.next().await {
                        if let Ok((_src, _dst, payload)) = decrypt_proxy_pdu(&sec_clone, iv, &notif.value) {
                            if let Some(decoded) = decode_telink_status(&payload) {
                                let mut s = status_clone.write().await;
                                s.connected = true;
                                s.connecting = false;
                                s.power = decoded.power;
                                s.brightness = decoded.brightness;
                                s.cct = decoded.cct;
                            }
                        }
                    }
                    // Stream closed (disconnection or power off at wall)
                    let mut s = status_clone.write().await;
                    s.connected = false;
                    s.connecting = false;
                });
            }
        }

        // Send proxy filter setup handshake
        let seq1 = self.next_seq();
        let pdu_filter = build_proxy_config_pdu(
            &self.security,
            seq1,
            LOCAL_SRC_ADDR,
            self.iv_index,
            PROXY_CFG_SET_FILTER_TYPE,
            &[PROXY_FILTER_WHITELIST],
        )?;
        let _ = peripheral.write(&char_in, &pdu_filter, WriteType::WithoutResponse).await;

        let seq2 = self.next_seq();
        let pdu_addr = build_proxy_config_pdu(
            &self.security,
            seq2,
            LOCAL_SRC_ADDR,
            self.iv_index,
            PROXY_CFG_ADD_ADDRESSES,
            &[(LOCAL_SRC_ADDR >> 8) as u8, (LOCAL_SRC_ADDR & 0xff) as u8],
        )?;
        let _ = peripheral.write(&char_in, &pdu_addr, WriteType::WithoutResponse).await;

        self.char_in = Some(char_in);
        self.peripheral = Some(peripheral);

        {
            let mut s = self.status.write().await;
            s.connected = true;
            s.connecting = false;
        }

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref p) = self.peripheral {
            let _ = p.disconnect().await;
        }
        self.peripheral = None;
        self.char_in = None;
        let mut s = self.status.write().await;
        s.connected = false;
        s.connecting = false;
        Ok(())
    }

    pub async fn send_raw_pdu(&mut self, pdu: &[u8]) -> Result<()> {
        self.ensure_connected().await?;
        let peripheral = self.peripheral.as_ref().context("not connected to light")?;
        let char_in = self.char_in.as_ref().context("characteristic not initialized")?;

        peripheral
            .write(char_in, pdu, WriteType::WithoutResponse)
            .await
            .context("failed to write BLE mesh packet")?;

        Ok(())
    }

    pub async fn poll_status(&mut self) -> Result<()> {
        let addr = self.config.primary_light().map(|l| l.address).unwrap_or(2);
        let pdu = self.build_status_request_pdu(addr)?;
        self.send_raw_pdu(&pdu).await
    }

    pub async fn set_power(&mut self, on: bool) -> Result<()> {
        let addr = self.config.primary_light().map(|l| l.address).unwrap_or(2);
        let pdu = self.build_onoff_pdu(addr, on)?;
        self.status.write().await.power = on;
        self.send_raw_pdu(&pdu).await
    }

    pub async fn set_brightness(&mut self, percent: f32) -> Result<()> {
        let addr = self.config.primary_light().map(|l| l.address).unwrap_or(2);
        let pdu = self.build_brightness_pdu(addr, percent)?;
        self.status.write().await.brightness = percent.clamp(0.0, 100.0);
        self.send_raw_pdu(&pdu).await
    }

    pub async fn set_brightness_raw(&mut self, raw: u16) -> Result<()> {
        let addr = self.config.primary_light().map(|l| l.address).unwrap_or(2);
        let pdu = self.build_brightness_raw_pdu(addr, raw)?;
        self.status.write().await.brightness = (raw.min(1000) as f32) / 10.0;
        self.send_raw_pdu(&pdu).await
    }

    pub async fn set_cct(&mut self, kelvin: u16) -> Result<()> {
        let addr = self.config.primary_light().map(|l| l.address).unwrap_or(2);
        let intensity = self.last_brightness;
        let pdu = self.build_cct_pdu(addr, kelvin, intensity, 0)?;
        self.status.write().await.cct = kelvin.clamp(2700, 6500);
        self.send_raw_pdu(&pdu).await
    }
}
