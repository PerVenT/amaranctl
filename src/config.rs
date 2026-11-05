use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightConfig {
    pub name: String,
    pub mac: String,
    pub address: u16,
    pub device_key: Option<String>,
    pub device_uuid: Option<String>,
    pub code: Option<String>,
}

impl LightConfig {
    pub fn device_key_bytes(&self) -> Option<Vec<u8>> {
        self.device_key.as_ref().and_then(|k| hex::decode(k).ok())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub mesh_uuid: String,
    pub net_key: String,
    pub app_key: String,
    pub lights: Vec<LightConfig>,
}

impl AppConfig {
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file at {:?}", path))?;
        Self::from_json_str(&content)
    }

    pub fn from_json_str(json: &str) -> Result<Self> {
        // Strip UTF-8 BOM if present (e.g. from Windows PowerShell Set-Content)
        let clean = json.strip_prefix('\u{feff}').unwrap_or(json).trim();
        if clean.is_empty() {
            anyhow::bail!("Config file is empty");
        }
        let config: Self = serde_json::from_str(clean)
            .context("Failed to deserialize AppConfig from JSON")?;
        Ok(config)
    }

    pub fn net_key_bytes(&self) -> Vec<u8> {
        hex::decode(&self.net_key).unwrap_or_default()
    }

    pub fn app_key_bytes(&self) -> Vec<u8> {
        hex::decode(&self.app_key).unwrap_or_default()
    }

    pub fn primary_light(&self) -> Option<&LightConfig> {
        self.lights.first()
    }

    pub fn find_amaran_db() -> Option<PathBuf> {
        // 1. Windows: %APPDATA%\amaran Desktop\*\amaran.db
        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let pattern = format!("{}\\amaran Desktop\\*\\amaran.db", appdata);
                if let Ok(entries) = glob::glob(&pattern) {
                    for entry in entries.flatten() {
                        if entry.is_file() {
                            return Some(entry);
                        }
                    }
                }
            }
        }

        // 2. Cross-platform / Unix standard paths
        if let Some(home) = dirs::home_dir() {
            let candidates = [
                home.join("Library/Application Support/amaran Desktop"),
                home.join(".config/amaran Desktop"),
                home.join(".wine/drive_c/users")
                    .join(std::env::var("USER").unwrap_or_default())
                    .join("AppData/Roaming/amaran Desktop"),
            ];

            for base in &candidates {
                if base.exists() {
                    let pattern = format!("{}/**/amaran.db", base.display());
                    if let Ok(entries) = glob::glob(&pattern) {
                        for entry in entries.flatten() {
                            if entry.is_file() {
                                return Some(entry);
                            }
                        }
                    }
                }
            }
        }

        // 3. Current working directory
        let local = Path::new("amaran.db");
        if local.is_file() {
            return Some(local.to_path_buf());
        }

        None
    }

    pub fn try_auto_import_from_db() -> Result<Self> {
        let db_path = Self::find_amaran_db()
            .context("Could not find amaran.db in Amaran Desktop application directories")?;

        let conn = rusqlite::Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        let (mesh_uuid, net_key, app_key): (String, String, String) = conn.query_row(
            "SELECT uuid, net_key, app_key FROM mesh LIMIT 1;",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).context("Failed to query mesh credentials from database")?;

        let mut stmt = conn.prepare("SELECT name, mac_address, node_address, device_key, device_uuid, code FROM fixtures;")
            .context("Failed to prepare fixtures query")?;

        let light_iter = stmt.query_map([], |row| {
            Ok(LightConfig {
                name: row.get(0)?,
                mac: row.get(1)?,
                address: row.get(2)?,
                device_key: row.get(3)?,
                device_uuid: row.get(4)?,
                code: row.get(5)?,
            })
        }).context("Failed to execute fixtures query")?;

        let mut lights = Vec::new();
        for light in light_iter {
            if let Ok(l) = light {
                lights.push(l);
            }
        }

        if lights.is_empty() {
            anyhow::bail!("No lights found in amaran.db fixtures table");
        }

        let config = AppConfig {
            mesh_uuid,
            net_key,
            app_key,
            lights,
        };

        // Automatically save to ~/.config/amaranctl/config.json for fast subsequent runs
        if let Some(home) = dirs::home_dir() {
            let target_dir = home.join(".config").join("amaranctl");
            let _ = fs::create_dir_all(&target_dir);
            let target_file = target_dir.join("config.json");
            if let Ok(json_str) = serde_json::to_string_pretty(&config) {
                let _ = fs::write(&target_file, json_str);
                eprintln!("[+] Auto-imported credentials from {:?} -> {:?}", db_path, target_file);
            }
        }

        Ok(config)
    }
}
