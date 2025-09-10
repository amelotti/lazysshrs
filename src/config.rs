use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub workdir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home_dir = home::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Self {
            workdir: home_dir.join(".ssh").to_string_lossy().to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = AppConfig::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = home::home_dir().ok_or("Could not find home directory")?;
        Ok(home_dir.join(".config").join("lazysshrs"))
    }

    pub fn get_main_config_path(&self) -> PathBuf {
        PathBuf::from(&self.workdir).join("config")
    }

    pub fn get_workdir(&self) -> PathBuf {
        PathBuf::from(&self.workdir)
    }
}