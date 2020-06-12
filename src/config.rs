use std::fs::{create_dir_all, write, File};
use std::io::Read;

use anyhow::format_err;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

use crate::Result;

static EXAMPLE_CONFIG: &str = "---
# Example config file for Grout

# Margin between windows, in pixels
margins: 10

# Padding between edge of monitor and windows, in pixels
window_padding: 10

# Hotkey to activate grid. Valid modifiers are CTRL, ALT, SHIFT, WIN
hotkey: CTRL+ALT+S

# Hotkey to activate grid for a quick resize. Grid will automatically close after resize operation.
#hotkey_quick_resize: CTRL+ALT+Q

# Hotkey to maximize / restore the active window
#hotkey_maximize_toggle: CTRL+ALT+X

# Automatically launch program on startup
auto_start: false
";

pub fn load_config() -> Result<Config> {
    let mut config_path =
        dirs::config_dir().ok_or_else(|| format_err!("Failed to get config directory"))?;
    config_path.push("grout");

    if !config_path.exists() {
        create_dir_all(&config_path)?;
    }

    config_path.push("config.yml");
    if !config_path.exists() {
        write(&config_path, EXAMPLE_CONFIG)?;
    }

    let mut config = config::Config::default();
    config.merge(config::Config::try_from(&Config::default()).unwrap())?;

    let file_config = config::File::from(config_path).format(config::FileFormat::Yaml);

    let config = config.merge(file_config)?;
    Ok(config.clone().try_into()?)
}

pub fn toggle_autostart() -> Result<()> {
    let mut config_path =
        dirs::config_dir().ok_or_else(|| format_err!("Failed to get config directory"))?;
    config_path.push("grout");
    config_path.push("config.yml");

    let mut config = File::open(&config_path)?;
    let mut config_str = String::new();

    config.read_to_string(&mut config_str)?;

    let re_line = Regex::new(r"(?m)^(auto_start:)(.*)$")?;
    let updated_config = if let Some(cap) = re_line.captures_iter(&config_str).next() {
        if re_line.captures_len() == 3 {
            let re_cap = Regex::new(r"(?m)^(y|Y|yes|Yes|YES|true|True|TRUE|on|On|ON)$")?;

            let enabled = re_cap.find(&cap[2].trim());

            let updated_config = re_line.replace(&config_str, |caps: &Captures| {
                format!("{} {}", &caps[1], !enabled.is_some())
            });

            Some(updated_config.as_ref().to_owned())
        } else {
            None
        }
    } else {
        None
    };

    let updated_config = if let Some(updated_config) = updated_config {
        updated_config
    } else {
        format!("{}\n\nauto_start: true", config_str)
    };

    write(&config_path, updated_config)?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub margins: u8,
    pub window_padding: u8,
    pub hotkey: String,
    pub hotkey_quick_resize: Option<String>,
    pub hotkey_maximize_toggle: Option<String>,
    pub auto_start: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            margins: 10,
            window_padding: 10,
            hotkey: "CTRL+ALT+S".to_string(),
            hotkey_quick_resize: None,
            hotkey_maximize_toggle: None,
            auto_start: false,
        }
    }
}
