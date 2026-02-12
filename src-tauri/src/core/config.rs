use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub max_record_seconds: u32,
    pub model: ModelSize,
    pub auto_type: bool,
    pub text_cleanup: bool,
    pub llm_postprocess_enabled: bool,
    pub llm_provider: String,
    pub llm_api_base_url: String,
    pub llm_api_key: String,
    pub llm_model: String,
    pub realtime_enabled: bool,
    pub partial_autotype_mode: PartialAutotypeMode,
    #[serde(default)]
    pub pill_position: Option<PillPosition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelSize {
    Small,
    Medium,
}

impl Default for ModelSize {
    fn default() -> Self {
        Self::Small
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartialAutotypeMode {
    Replace,
}

impl Default for PartialAutotypeMode {
    fn default() -> Self {
        Self::Replace
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PillPosition {
    pub x: i32,
    pub y: i32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_record_seconds: 60,
            model: ModelSize::Small,
            auto_type: true,
            text_cleanup: true,
            llm_postprocess_enabled: false,
            llm_provider: "".to_string(),
            llm_api_base_url: "".to_string(),
            llm_api_key: "".to_string(),
            llm_model: "".to_string(),
            realtime_enabled: true,
            partial_autotype_mode: PartialAutotypeMode::Replace,
            pill_position: None,
        }
    }
}

fn config_path() -> anyhow::Result<PathBuf> {
    if let Ok(dir) = std::env::var("NOTYPE_CONFIG_DIR") {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir)?;
        return Ok(dir.join("config.json"));
    }

    let dirs = ProjectDirs::from("dev", "notype", "notype")
        .ok_or_else(|| anyhow::anyhow!("could not resolve config dir"))?;
    let dir = dirs.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn default_is_small_and_realtime() {
        let cfg = AppConfig::default();
        assert!(matches!(cfg.model, ModelSize::Small));
        assert!(cfg.realtime_enabled);
    }

    #[test]
    fn broken_config_recovers_to_default() {
        let temp = std::env::temp_dir().join(format!("notype-test-{}", Uuid::new_v4()));
        std::env::set_var("NOTYPE_CONFIG_DIR", temp.display().to_string());
        std::fs::create_dir_all(&temp).expect("mkdir");
        std::fs::write(temp.join("config.json"), "{invalid").expect("write");

        let cfg = load_config().expect("load with recovery");
        assert!(matches!(cfg.model, ModelSize::Small));
        assert_eq!(cfg.max_record_seconds, 60);
        assert_eq!(cfg.pill_position, None);
    }

    #[test]
    fn pill_position_roundtrip() {
        let temp = std::env::temp_dir().join(format!("notype-test-{}", Uuid::new_v4()));
        std::env::set_var("NOTYPE_CONFIG_DIR", temp.display().to_string());
        std::fs::create_dir_all(&temp).expect("mkdir");

        let mut cfg = AppConfig::default();
        cfg.pill_position = Some(PillPosition { x: 320, y: 48 });
        save_config(&cfg).expect("save");

        let loaded = load_config().expect("load");
        assert_eq!(loaded.pill_position, Some(PillPosition { x: 320, y: 48 }));
    }
}

pub fn load_config() -> anyhow::Result<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        let cfg = AppConfig::default();
        save_config(&cfg)?;
        return Ok(cfg);
    }

    let raw = fs::read_to_string(&path)?;
    match serde_json::from_str::<AppConfig>(&raw) {
        Ok(cfg) => Ok(cfg),
        Err(_) => {
            // Recover from broken config by regenerating defaults.
            let cfg = AppConfig::default();
            save_config(&cfg)?;
            Ok(cfg)
        }
    }
}

pub fn save_config(cfg: &AppConfig) -> anyhow::Result<()> {
    let path = config_path()?;
    let raw = serde_json::to_string_pretty(cfg)?;
    fs::write(path, raw)?;
    Ok(())
}
