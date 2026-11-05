use anyhow::{Context, Result};
use global_hotkey::HotKeyState;
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tokio::sync::mpsc;
#[cfg(not(windows))]
use std::time::Duration;
use tray_icon::{Icon, TrayIconBuilder};

use crate::autostart::{is_autostart_enabled, set_autostart};
use crate::ble::BleMeshClient;
use crate::config::AppConfig;
use crate::hotkeys::HotkeyBindings;

pub enum AppAction {
    SetPower(bool),
    TogglePower,
    SetBrightness(f32),
    AdjustBrightness(f32),
    SetCct(u16),
    AdjustCct(i32),
    Quit,
}

pub fn create_default_icon() -> Icon {
    let width = 32u32;
    let height = 32u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32) - 15.5;
            let dy = (y as f32) - 15.5;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= 12.0 {
                // Golden warm lightbulb color (RGBA: 255, 204, 0, 255)
                rgba.extend_from_slice(&[255, 204, 0, 255]);
            } else if dist <= 13.5 {
                // Anti-aliased border
                let alpha = ((13.5 - dist) / 1.5 * 255.0) as u8;
                rgba.extend_from_slice(&[255, 204, 0, alpha]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    Icon::from_rgba(rgba, width, height).expect("valid 32x32 icon")
}

pub fn run_tray_app(config: AppConfig) -> Result<()> {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::FreeConsole();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = gtk::init();
    }

    let icon = create_default_icon();
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

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("amaranctl - Amaran 60x S Controller")
        .with_icon(icon)
        .build()
        .context("failed to build tray icon")?;

    let hotkeys = HotkeyBindings::new().ok();

    // Spawn async background worker thread for BLE
    let (tx, mut rx) = mpsc::unbounded_channel::<AppAction>();
    let tx_worker = tx.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async move {
            let mut client = match BleMeshClient::new(config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error creating BLE client: {:?}", e);
                    return;
                }
            };

            // Attempt initial connection in background
            let _ = client.ensure_connected().await;

            // Spawn IPC server concurrently with BLE loop
            let tx_ipc = tx_worker.clone();
            tokio::spawn(async move {
                use crate::ipc::{IpcCommand, IpcResponse, IpcServer};
                let _ = IpcServer::run(move |cmd| {
                    let tx = tx_ipc.clone();
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
                            IpcCommand::Ping => {
                                IpcResponse { ok: true, message: Some("pong".into()) }
                            }
                        }
                    }
                }).await;
            });

            while let Some(action) = rx.recv().await {
                match action {
                    AppAction::SetPower(on) => {
                        let _ = client.set_power(on).await;
                    }
                    AppAction::TogglePower => {
                        let new_state = !client.last_power;
                        let _ = client.set_power(new_state).await;
                    }
                    AppAction::SetBrightness(val) => {
                        let _ = client.set_brightness(val).await;
                    }
                    AppAction::AdjustBrightness(delta) => {
                        let cur_percent = (client.last_brightness as f32) / 10.0;
                        let new_val = (cur_percent + delta).clamp(0.0, 100.0);
                        let _ = client.set_brightness(new_val).await;
                    }
                    AppAction::SetCct(k) => {
                        let _ = client.set_cct(k).await;
                    }
                    AppAction::AdjustCct(delta) => {
                        let cur = client.last_cct as i32;
                        let new_k = (cur + delta).clamp(2700, 6500) as u16;
                        let _ = client.set_cct(new_k).await;
                    }
                    AppAction::Quit => break,
                }
            }
        });
    });

    let menu_receiver = MenuEvent::receiver();
    let hotkey_receiver = global_hotkey::GlobalHotKeyEvent::receiver();

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
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
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
                        "toggle" => { let _ = tx.send(AppAction::TogglePower); }
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
                    "toggle" => { let _ = tx.send(AppAction::TogglePower); }
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

            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
