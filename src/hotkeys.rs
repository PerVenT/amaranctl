use anyhow::{Context, Result};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

pub struct HotkeyBindings {
    pub manager: GlobalHotKeyManager,
    pub toggle_id: u32,
    pub bright_up_id: u32,
    pub bright_down_id: u32,
    pub cct_up_id: u32,
    pub cct_down_id: u32,
}

impl HotkeyBindings {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("failed to init GlobalHotKeyManager")?;

        let toggle_key = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyL);
        let bright_up_key = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowUp);
        let bright_down_key = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowDown);
        let cct_up_key = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowRight);
        let cct_down_key = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowLeft);

        let toggle_id = toggle_key.id();
        let bright_up_id = bright_up_key.id();
        let bright_down_id = bright_down_key.id();
        let cct_up_id = cct_up_key.id();
        let cct_down_id = cct_down_key.id();

        manager.register(toggle_key).context("register toggle hotkey")?;
        manager.register(bright_up_key).context("register bright up hotkey")?;
        manager.register(bright_down_key).context("register bright down hotkey")?;
        manager.register(cct_up_key).context("register cct up hotkey")?;
        manager.register(cct_down_key).context("register cct down hotkey")?;

        Ok(Self {
            manager,
            toggle_id,
            bright_up_id,
            bright_down_id,
            cct_up_id,
            cct_down_id,
        })
    }
}
