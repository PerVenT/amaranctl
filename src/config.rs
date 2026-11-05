use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
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
        let config: Self = serde_json::from_str(json)
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
}
