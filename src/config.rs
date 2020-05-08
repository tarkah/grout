use std::fs;

use serde::{Deserialize, Serialize};

static EXAMPLE_CONFIG: &str = "---
# Example config file for Grout

# Margin between windows, in pixels
margins: 10

# Padding between edge of monitor and windows, in pixels
window_padding: 10
";

pub fn load_config() -> Config {
    if let Some(mut config_path) = dirs::config_dir() {
        config_path.push("grout");
        if !config_path.exists() {
            let _ = fs::create_dir_all(&config_path);
        }

        config_path.push("config.yml");
        if !config_path.exists() {
            let _ = fs::write(&config_path, EXAMPLE_CONFIG);
        }

        let mut config = config::Config::default();
        let _ = config.merge(config::Config::try_from(&Config::default()).unwrap());

        let file_config = config::File::from(config_path).format(config::FileFormat::Yaml);

        if let Ok(config) = config.merge(file_config) {
            return config.clone().try_into().unwrap_or_default();
        }
    };

    Config::default()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub margins: u8,
    pub window_padding: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            margins: 10,
            window_padding: 10,
        }
    }
}
