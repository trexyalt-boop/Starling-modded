//! Persistent configuration for the Starling Native Mod Menu.
//!
//! Settings are serialized to and deserialized from `modmenu-settings.json`
//! in the active game directory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CONFIG_FILENAME: &str = "modmenu-settings.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModConfig {
    /// Whether the menu is currently visible.
    pub menu_open: bool,
    /// X position of the floating mod menu window (GUI pixels).
    pub window_x: f32,
    /// Y position of the floating mod menu window (GUI pixels).
    pub window_y: f32,
    /// Width of the mod menu window.
    pub window_w: f32,
    /// Height of the mod menu window.
    pub window_h: f32,
    /// Position of the floating open/close toggle button (GUI pixels).
    pub button_x: f32,
    pub button_y: f32,
    /// Currently selected category index (0..6).
    pub active_category: usize,
    /// Feature enabled states by feature ID.
    pub features_enabled: HashMap<String, bool>,
    /// Float settings by "feature_id:setting_key".
    pub float_settings: HashMap<String, f32>,
    /// Bool settings by "feature_id:setting_key".
    pub bool_settings: HashMap<String, bool>,
    /// Enum selection index by "feature_id:setting_key".
    pub enum_settings: HashMap<String, usize>,
    /// Color settings by "feature_id:setting_key" as [r, g, b, a].
    pub color_settings: HashMap<String, [u8; 4]>,
}

impl Default for ModConfig {
    fn default() -> Self {
        Self {
            menu_open: true, // Automatically show menu on start as required
            window_x: 40.0,
            window_y: 30.0,
            window_w: 360.0,
            window_h: 230.0,
            button_x: 10.0,
            button_y: 10.0,
            active_category: 0,
            features_enabled: HashMap::new(),
            float_settings: HashMap::new(),
            bool_settings: HashMap::new(),
            enum_settings: HashMap::new(),
            color_settings: HashMap::new(),
        }
    }
}

impl ModConfig {
    /// Loads configuration from `modmenu-settings.json` or returns default.
    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<ModConfig>(&data) {
                return config;
            }
        }
        Self::default()
    }

    /// Saves configuration to disk atomically or directly.
    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Helper to determine the path to `modmenu-settings.json`.
    fn config_path() -> PathBuf {
        if let Some(home) = std::env::var_os("HORIZON_HOME") {
            Path::new(&home).join(CONFIG_FILENAME)
        } else {
            PathBuf::from(CONFIG_FILENAME)
        }
    }

    pub fn is_feature_enabled(&self, id: &str, default_value: bool) -> bool {
        *self.features_enabled.get(id).unwrap_or(&default_value)
    }

    pub fn set_feature_enabled(&mut self, id: &str, enabled: bool) {
        self.features_enabled.insert(id.to_string(), enabled);
    }

    pub fn get_float(&self, key: &str, default_value: f32) -> f32 {
        *self.float_settings.get(key).unwrap_or(&default_value)
    }

    pub fn set_float(&mut self, key: &str, val: f32) {
        self.float_settings.insert(key.to_string(), val);
    }

    pub fn get_bool(&self, key: &str, default_value: bool) -> bool {
        *self.bool_settings.get(key).unwrap_or(&default_value)
    }

    pub fn set_bool(&mut self, key: &str, val: bool) {
        self.bool_settings.insert(key.to_string(), val);
    }

    pub fn get_enum(&self, key: &str, default_value: usize) -> usize {
        *self.enum_settings.get(key).unwrap_or(&default_value)
    }

    pub fn set_enum(&mut self, key: &str, val: usize) {
        self.enum_settings.insert(key.to_string(), val);
    }

    pub fn get_color(&self, key: &str, default_value: [u8; 4]) -> [u8; 4] {
        *self.color_settings.get(key).unwrap_or(&default_value)
    }

    pub fn set_color(&mut self, key: &str, val: [u8; 4]) {
        self.color_settings.insert(key.to_string(), val);
    }
}
