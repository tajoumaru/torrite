/// Block size for V2 hashing (16 KiB)
pub const BLOCK_SIZE: usize = 16384;

/// Megabyte constant for piece length calculations
pub const MB: u64 = 1_048_576;

/// Piece length thresholds for automatic calculation
/// Maps total size to piece length power (2^N)
pub const PIECE_LENGTH_THRESHOLDS: [(u64, u32); 9] = [
    (50 * MB, 15),    // <=50MB   -> 2^15 (32 KB)
    (100 * MB, 16),   // <=100MB  -> 2^16 (64 KB)
    (200 * MB, 17),   // <=200MB  -> 2^17 (128 KB)
    (400 * MB, 18),   // <=400MB  -> 2^18 (256 KB)
    (800 * MB, 19),   // <=800MB  -> 2^19 (512 KB)
    (1600 * MB, 20),  // <=1.6GB  -> 2^20 (1 MB)
    (3200 * MB, 21),  // <=3.2GB  -> 2^21 (2 MB)
    (6400 * MB, 22),  // <=6.4GB  -> 2^22 (4 MB)
    (12800 * MB, 23), // <=12.8GB -> 2^23 (8 MB)
];

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::Deserialize;
use directories::ProjectDirs;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Profile {
    pub announce: Option<Vec<String>>,
    
    #[serde(rename = "source")]
    pub source_string: Option<String>,
    
    pub comment: Option<String>,
    pub private: Option<bool>,
    
    #[serde(rename = "piece_length")]
    pub piece_length: Option<u32>,
    
    pub threads: Option<usize>,
    
    #[serde(rename = "web_seed")]
    pub web_seed: Option<Vec<String>>,
    
    #[serde(rename = "cross_seed")]
    pub cross_seed: Option<bool>,
    
    pub v2: Option<bool>,
    pub hybrid: Option<bool>,
    
    pub exclude: Option<Vec<String>>,
    
    #[serde(rename = "no_date")]
    pub no_date: Option<bool>,
}

impl Config {
    pub fn load(cli_path: Option<PathBuf>) -> Result<Self> {
        // 1. CLI Arguments
        if let Some(path) = cli_path {
            if path.exists() {
                return Self::from_file(&path);
            } else {
                 return Err(anyhow::anyhow!("Config file not found: {}", path.display()));
            }
        }

        // 2. Environment Variables
        if let Ok(path_str) = std::env::var("TORRITE_CONFIG_PATH") {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Self::from_file(&path);
            }
        }

        // 3. Local File
        let local_path = Path::new("torrite.toml");
        if local_path.exists() {
            return Self::from_file(local_path);
        }

        // 4. Global Config
        if let Some(proj_dirs) = ProjectDirs::from("", "", "torrite") {
            let config_dir = proj_dirs.config_dir();
            let global_path = config_dir.join("config.toml");
            if global_path.exists() {
                return Self::from_file(&global_path);
            }
        }

        // Return default config if no file found
        Ok(Config::default())
    }

    fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_simple_config() {
        let toml_content = r#"
            [profiles.ptp]
            announce = ["https://ptp.tracker"]
            source = "PTP"
            piece_length = 18
            
            [profiles.default]
            threads = 4
        "#;
        
        let config: Config = toml::from_str(toml_content).unwrap();
        
        assert!(config.profiles.contains_key("ptp"));
        let ptp = &config.profiles["ptp"];
        assert_eq!(ptp.source_string, Some("PTP".to_string()));
        assert_eq!(ptp.piece_length, Some(18));
        assert_eq!(ptp.announce.as_ref().unwrap()[0], "https://ptp.tracker");
        
        assert!(config.profiles.contains_key("default"));
        assert_eq!(config.profiles["default"].threads, Some(4));
    }

    #[test]
    fn test_load_from_file() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, r#"
            [profiles.test]
            comment = "Test Profile"
        "#)?;
        
        let config = Config::from_file(file.path())?;
        assert!(config.profiles.contains_key("test"));
        assert_eq!(config.profiles["test"].comment, Some("Test Profile".to_string()));
        
        Ok(())
    }
}