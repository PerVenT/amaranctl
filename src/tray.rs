use anyhow::Result;
use global_hotkey::HotKeyState;
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tray_icon::{Icon, TrayIconBuilder};

use crate::autostart::{is_autostart_enabled, set_autostart};
use crate::ble::{BleMeshClient, LampStatus};
use crate::config::AppConfig;
use crate::hotkeys::HotkeyBindings;

pub enum AppAction {
    SetPower(bool),
    TogglePower,
    SetBrightness(f32),
    AdjustBrightness(f32),
    SetCct(u16),
    AdjustCct(i32),
    PollStatus,
    Quit,
}

pub fn cct_to_rgb(kelvin: u16) -> (u8, u8, u8) {
    let k = kelvin.clamp(2700, 6500);
    if k <= 4500 {
        let t = (k - 2700) as f32 / (4500 - 2700) as f32;
        let r = (255.0 + (255.0 - 255.0) * t).round() as u8;
        let g = (170.0 + (244.0 - 170.0) * t).round() as u8;
        let b = (51.0 + (224.0 - 51.0) * t).round() as u8;
        (r, g, b)
    } else {
        let t = (k - 4500) as f32 / (6500 - 4500) as f32;
        let r = (255.0 + (204.0 - 255.0) * t).round() as u8;
        let g = (244.0 + (230.0 - 244.0) * t).round() as u8;
        let b = (224.0 + (255.0 - 224.0) * t).round() as u8;
        (r, g, b)
    }
}

pub fn create_adaptive_icon(status: &LampStatus, frame: u32) -> Icon {
    let width = 32u32;
    let height = 32u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    let (base_r, base_g, base_b) = if status.connecting {
        let val = match frame % 4 {
            0 => 100u8,
            1 => 180u8,
            2 => 255u8,
            _ => 180u8,
        };
        (val, val, val)
    } else if !status.connected {
        // Disconnected: Dim gray
        (60, 60, 60)
    } else if !status.power {
        // Connected OFF: Dim gray
        (60, 60, 60)
    } else {
        // Connected ON: CCT-tinted color
        cct_to_rgb(status.cct)
    };

    let show_disconnect_dot = !status.connected && !status.connecting;

    for y in 0..height {
        for x in 0..width {
            // Check for red disconnect badge at bottom right
            if show_disconnect_dot {
                let dx_dot = (x as f32) - 23.5;
                let dy_dot = (y as f32) - 23.5;
                let dist_dot = (dx_dot * dx_dot + dy_dot * dy_dot).sqrt();
                if dist_dot <= 4.0 {
                    rgba.extend_from_slice(&[231, 76, 60, 255]);
                    continue;
                } else if dist_dot <= 5.0 {
                    let alpha = ((5.0 - dist_dot) * 255.0) as u8;
                    rgba.extend_from_slice(&[231, 76, 60, alpha]);
                    continue;
                }
            }

            let dx = (x as f32) - 15.5;
            let dy = (y as f32) - 15.5;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= 12.0 {
                rgba.extend_from_slice(&[base_r, base_g, base_b, 255]);
            } else if dist <= 13.5 {
                let alpha = ((13.5 - dist) / 1.5 * 255.0) as u8;
                rgba.extend_from_slice(&[base_r, base_g, base_b, alpha]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    Icon::from_rgba(rgba, width, height).expect("valid 32x32 icon")
}

pub fn create_default_icon() -> Icon {
    let status = LampStatus::default();
    create_adaptive_icon(&status, 0)
}

pub fn run_tray_app(config: AppConfig) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let _ = gtk::init();
    }

    let shared_status = Arc::new(RwLock::new(LampStatus::default()));
    let initial_status = LampStatus::default();
    let icon = create_adaptive_icon(&initial_status, 0);
    let tray_menu = Menu::new();

    let header = MenuItem::with_id("status", "amaranctl: Amaran 60x S", false, None);
    let sep1 = PredefinedMenuItem::separator();
    let toggle = MenuItem::with_id("toggle", "Toggle Power (Ctrl+Alt+L)", true, None);

    let brightness_menu = Submenu::new("Brightness", true);
    brightness_menu.append(&MenuItem::with_id("b_10", "10%", true, None))?;
    brightness_menu.append(&MenuItem::with_id("b_25", "25%", true, None))?;
    brightness_menu.append(&MenuItem::with_id("b_50", "50%", true, None))?;
    brightness_menu.append(&MenuItem::with_id("b_75", "75%", true, None))?;
    brightness_menu.append(&MenuItem::with_id("b_100", "100%", true, None))?;

    let cct_menu = Submenu::new("Color Temp (CCT)", true);
    cct_menu.append(&MenuItem::with_id("c_2700", "2700K (Warm)", true, None))?;
    cct_menu.append(&MenuItem::with_id("c_3200", "3200K (Tungsten)", true, None))?;
    cct_menu.append(&MenuItem::with_id("c_4500", "4500K (Neutral)", true, None))?;
    cct_menu.append(&MenuItem::with_id("c_5600", "5600K (Daylight)", true, None))?;
    cct_menu.append(&MenuItem::with_id("c_6500", "6500K (Cool)", true, None))?;

    let sep2 = PredefinedMenuItem::separator();
    let autostart_enabled = is_autostart_enabled();
    let autostart_item = CheckMenuItem::with_id(
        "autostart",
        "Autostart on Boot",
        true,
        autostart_enabled,
        None,
    );
    let sep3 = PredefinedMenuItem::separator();
    let quit = MenuItem::with_id("quit", "Quit", true, None);

    tray_menu.append(&header)?;
    tray_menu.append(&sep1)?;
    tray_menu.append(&toggle)?;
    tray_menu.append(&brightness_menu)?;
    tray_menu.append(&cct_menu)?;
    tray_menu.append(&sep2)?;
    tray_menu.append(&autostart_item)?;
    tray_menu.append(&sep3)?;
    tray_menu.append(&quit)?;

    let tray_icon = match TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(initial_status.tooltip_text())
        .with_icon(icon)
        .build()
    {
        Ok(ti) => Some(ti),
        Err(e) => {
            eprintln!("[warn] System tray icon could not be created ({:?}). Running in background daemon mode.", e);
            None
        }
    };

    let hotkeys = HotkeyBindings::new().ok();

    // Spawn async background worker thread for BLE
    let (tx, mut rx) = mpsc::unbounded_channel::<AppAction>();
    let tx_worker = tx.clone();
    let shared_status_worker = shared_status.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async move {
            let mut client = match BleMeshClient::new(config) {
                Ok(mut c) => {
                    c.status = shared_status_worker.clone();
                    c
                }
                Err(e) => {
                    eprintln!("Error creating BLE client: {:?}", e);
                    return;
                }
            };

            // Spawn IPC server IMMEDIATELY so IPC is available from millisecond 1
            let tx_ipc = tx_worker.clone();
            let shared_status_ipc = shared_status_worker.clone();
            tokio::spawn(async move {
                use crate::ipc::{IpcCommand, IpcResponse, IpcServer};
                let _ = IpcServer::run(move |cmd| {
                    let tx = tx_ipc.clone();
                    let status_ref = shared_status_ipc.clone();
                    async move {
                        match cmd {
                            IpcCommand::On => {
                                let _ = tx.send(AppAction::SetPower(true));
                                IpcResponse { ok: true, message: Some("Power ON".into()) }
                            }
                            IpcCommand::Off => {
                                let _ = tx.send(AppAction::SetPower(false));
                                IpcResponse { ok: true, message: Some("Power OFF".into()) }
                            }
                            IpcCommand::Toggle => {
                                let _ = tx.send(AppAction::TogglePower);
                                IpcResponse { ok: true, message: Some("Power TOGGLED".into()) }
                            }
                            IpcCommand::Brightness(val) => {
                                let _ = tx.send(AppAction::SetBrightness(val));
                                IpcResponse { ok: true, message: Some(format!("Brightness set to {:.1}%", val)) }
                            }
                            IpcCommand::Cct(val) => {
                                let _ = tx.send(AppAction::SetCct(val));
                                IpcResponse { ok: true, message: Some(format!("CCT set to {}K", val)) }
                            }
                            IpcCommand::Status => {
                                let s = status_ref.read().await.clone();
                                let json_str = serde_json::to_string(&s).unwrap_or_else(|_| "{}".into());
                                IpcResponse { ok: true, message: Some(json_str) }
                            }
                            IpcCommand::Ping => {
                                IpcResponse { ok: true, message: Some("pong".into()) }
                            }
                        }
                    }
                }).await;
            });

            // Attempt initial connection in background
            let _ = client.ensure_connected().await;

            let mut poll_interval = tokio::time::interval(Duration::from_secs(2));

            loop {
                tokio::select! {
                    _ = poll_interval.tick() => {
                        if client.is_connected().await {
                            let _ = client.poll_status().await;
                        }
                    }
                    Some(action) = rx.recv() => {
                        match action {
                            AppAction::SetPower(on) => {
                                let _ = client.set_power(on).await;
                            }
                            AppAction::TogglePower => {
                                let cur_power = client.status.read().await.power;
                                let _ = client.set_power(!cur_power).await;
                            }
                            AppAction::SetBrightness(val) => {
                                let _ = client.set_brightness(val).await;
                            }
                            AppAction::AdjustBrightness(delta) => {
                                let cur_percent = client.status.read().await.brightness;
                                let new_val = (cur_percent + delta).clamp(0.0, 100.0);
                                let _ = client.set_brightness(new_val).await;
                            }
                            AppAction::SetCct(k) => {
                                let _ = client.set_cct(k).await;
                            }
                            AppAction::AdjustCct(delta) => {
                                let cur = client.status.read().await.cct as i32;
                                let new_k = (cur + delta).clamp(2700, 6500) as u16;
                                let _ = client.set_cct(new_k).await;
                            }
                            AppAction::PollStatus => {
                                let _ = client.poll_status().await;
                            }
                            AppAction::Quit => break,
                        }
                    }
                }
            }
        });
    });

    let menu_receiver = MenuEvent::receiver();
    let hotkey_receiver = global_hotkey::GlobalHotKeyEvent::receiver();

    let mut last_rendered_status = LampStatus::default();
    let mut anim_frame: u32 = 0;
    let mut last_anim_tick = Instant::now();

    // Platform message loop
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, TranslateMessage, MSG,
            PM_REMOVE, QS_ALLINPUT,
        };

        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            loop {
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) > 0 {
                    if msg.message == windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT {
                        let _ = tx.send(AppAction::Quit);
                        return Ok(());
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                // Handle Menu Events
                while let Ok(event) = menu_receiver.try_recv() {
                    match event.id.0.as_str() {
                        "toggle" => {
                            let is_connecting = shared_status.try_read().map(|s| s.connecting).unwrap_or(false);
                            if !is_connecting {
                                let _ = tx.send(AppAction::TogglePower);
                            }
                        }
                        "b_10" => { let _ = tx.send(AppAction::SetBrightness(10.0)); }
                        "b_25" => { let _ = tx.send(AppAction::SetBrightness(25.0)); }
                        "b_50" => { let _ = tx.send(AppAction::SetBrightness(50.0)); }
                        "b_75" => { let _ = tx.send(AppAction::SetBrightness(75.0)); }
                        "b_100" => { let _ = tx.send(AppAction::SetBrightness(100.0)); }
                        "c_2700" => { let _ = tx.send(AppAction::SetCct(2700)); }
                        "c_3200" => { let _ = tx.send(AppAction::SetCct(3200)); }
                        "c_4500" => { let _ = tx.send(AppAction::SetCct(4500)); }
                        "c_5600" => { let _ = tx.send(AppAction::SetCct(5600)); }
                        "c_6500" => { let _ = tx.send(AppAction::SetCct(6500)); }
                        "autostart" => {
                            let new_state = !is_autostart_enabled();
                            let _ = set_autostart(new_state);
                            autostart_item.set_checked(is_autostart_enabled());
                        }
                        "quit" => {
                            let _ = tx.send(AppAction::Quit);
                            return Ok(());
                        }
                        _ => {}
                    }
                }

                // Handle Hotkey Events
                if let Some(ref hk) = hotkeys {
                    while let Ok(event) = hotkey_receiver.try_recv() {
                        if event.state != HotKeyState::Pressed {
                            continue;
                        }
                        if event.id == hk.toggle_id {
                            let _ = tx.send(AppAction::TogglePower);
                        } else if event.id == hk.bright_up_id {
                            let _ = tx.send(AppAction::AdjustBrightness(10.0));
                        } else if event.id == hk.bright_down_id {
                            let _ = tx.send(AppAction::AdjustBrightness(-10.0));
                        } else if event.id == hk.cct_up_id {
                            let _ = tx.send(AppAction::AdjustCct(500));
                        } else if event.id == hk.cct_down_id {
                            let _ = tx.send(AppAction::AdjustCct(-500));
                        }
                    }
                }

                // Reactive icon & tooltip update
                if let Ok(guard) = shared_status.try_read() {
                    let cur = guard.clone();
                    let is_connecting = cur.connecting;
                    let should_update_anim = is_connecting && last_anim_tick.elapsed() >= Duration::from_millis(400);
                    let status_changed = cur != last_rendered_status;

                    if status_changed || should_update_anim {
                        if should_update_anim {
                            anim_frame = anim_frame.wrapping_add(1);
                            last_anim_tick = Instant::now();
                        }
                        if let Some(ref ti) = tray_icon {
                            let _ = ti.set_icon(Some(create_adaptive_icon(&cur, anim_frame)));
                            let _ = ti.set_tooltip(Some(cur.tooltip_text()));
                        }
                        last_rendered_status = cur;
                    }
                }

                MsgWaitForMultipleObjects(0, std::ptr::null(), 0, 50, QS_ALLINPUT);
            }
        }
    }

    #[cfg(not(windows))]
    {
        loop {
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }

            while let Ok(event) = menu_receiver.try_recv() {
                match event.id.0.as_str() {
                    "toggle" => {
                        let is_connecting = shared_status.try_read().map(|s| s.connecting).unwrap_or(false);
                        if !is_connecting {
                            let _ = tx.send(AppAction::TogglePower);
                        }
                    }
                    "b_10" => { let _ = tx.send(AppAction::SetBrightness(10.0)); }
                    "b_25" => { let _ = tx.send(AppAction::SetBrightness(25.0)); }
                    "b_50" => { let _ = tx.send(AppAction::SetBrightness(50.0)); }
                    "b_75" => { let _ = tx.send(AppAction::SetBrightness(75.0)); }
                    "b_100" => { let _ = tx.send(AppAction::SetBrightness(100.0)); }
                    "c_2700" => { let _ = tx.send(AppAction::SetCct(2700)); }
                    "c_3200" => { let _ = tx.send(AppAction::SetCct(3200)); }
                    "c_4500" => { let _ = tx.send(AppAction::SetCct(4500)); }
                    "c_5600" => { let _ = tx.send(AppAction::SetCct(5600)); }
                    "c_6500" => { let _ = tx.send(AppAction::SetCct(6500)); }
                    "autostart" => {
                        let new_state = !is_autostart_enabled();
                        let _ = set_autostart(new_state);
                        autostart_item.set_checked(is_autostart_enabled());
                    }
                    "quit" => {
                        let _ = tx.send(AppAction::Quit);
                        return Ok(());
                    }
                    _ => {}
                }
            }

            if let Some(ref hk) = hotkeys {
                while let Ok(event) = hotkey_receiver.try_recv() {
                    if event.state != HotKeyState::Pressed {
                        continue;
                    }
                    if event.id == hk.toggle_id {
                        let _ = tx.send(AppAction::TogglePower);
                    } else if event.id == hk.bright_up_id {
                        let _ = tx.send(AppAction::AdjustBrightness(10.0));
                    } else if event.id == hk.bright_down_id {
                        let _ = tx.send(AppAction::AdjustBrightness(-10.0));
                    } else if event.id == hk.cct_up_id {
                        let _ = tx.send(AppAction::AdjustCct(500));
                    } else if event.id == hk.cct_down_id {
                        let _ = tx.send(AppAction::AdjustCct(-500));
                    }
                }
            }

            // Reactive icon & tooltip update
            if let Ok(guard) = shared_status.try_read() {
                let cur = guard.clone();
                let is_connecting = cur.connecting;
                let should_update_anim = is_connecting && last_anim_tick.elapsed() >= Duration::from_millis(400);
                let status_changed = cur != last_rendered_status;

                if status_changed || should_update_anim {
                    if should_update_anim {
                        anim_frame = anim_frame.wrapping_add(1);
                        last_anim_tick = Instant::now();
                    }
                    if let Some(ref ti) = tray_icon {
                        let _ = ti.set_icon(Some(create_adaptive_icon(&cur, anim_frame)));
                        let _ = ti.set_tooltip(Some(cur.tooltip_text()));
                    }
                    last_rendered_status = cur;
                }
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}
