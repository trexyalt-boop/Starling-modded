//! Persistent configuration for the Starling Corz menu.
//!
//! Everything is serialized to `modmenu-settings.json` in the active game
//! directory. Every field added after the first release carries
//! `#[serde(default)]`, so an older file keeps loading: the Borizon menu's
//! `features_enabled` / `float_settings` / `bool_settings` / `enum_settings`
//! / `color_settings` keys still apply to the features that kept their ids.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CONFIG_FILENAME: &str = "modmenu-settings.json";

fn d_true() -> bool {
    true
}

fn d_opacity() -> f32 {
    0.86
}

fn d_ui_scale() -> f32 {
    1.0
}

fn d_accent() -> [u8; 4] {
    [124, 92, 255, 255]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModConfig {
    /// Whether the menu is currently visible.
    #[serde(default = "d_true")]
    pub menu_open: bool,
    /// X position of the menu window (GUI pixels).
    #[serde(default = "d_win_x")]
    pub window_x: f32,
    #[serde(default = "d_win_y")]
    pub window_y: f32,
    #[serde(default = "d_win_w")]
    pub window_w: f32,
    #[serde(default = "d_win_h")]
    pub window_h: f32,
    /// Position of the floating open/close button (GUI pixels).
    #[serde(default = "d_btn_x")]
    pub button_x: f32,
    #[serde(default = "d_btn_y")]
    pub button_y: f32,
    /// Currently selected category index.
    #[serde(default)]
    pub active_category: usize,
    /// Feature enabled states by feature ID.
    #[serde(default)]
    pub features_enabled: HashMap<String, bool>,
    /// Float settings by "feature_id:setting_key".
    #[serde(default)]
    pub float_settings: HashMap<String, f32>,
    /// Bool settings by "feature_id:setting_key".
    #[serde(default)]
    pub bool_settings: HashMap<String, bool>,
    /// Enum selection index by "feature_id:setting_key".
    #[serde(default)]
    pub enum_settings: HashMap<String, usize>,
    /// Color settings by "feature_id:setting_key" as [r, g, b, a].
    #[serde(default)]
    pub color_settings: HashMap<String, [u8; 4]>,
    /// Integer settings by "feature_id:setting_key".
    #[serde(default)]
    pub int_settings: HashMap<String, i32>,
    /// Key bindings by "feature_id:setting_key" (a winit key name).
    #[serde(default)]
    pub keybind_settings: HashMap<String, String>,

    // --- Appearance (GUI Settings) ---
    /// Accent colour of the whole interface.
    #[serde(default = "d_accent")]
    pub accent: [u8; 4],
    /// Palette preset index (0 = Custom).
    #[serde(default)]
    pub theme_index: usize,
    /// Frosted blur behind the window.
    #[serde(default = "d_true")]
    pub blur: bool,
    /// Open/close, hover and category animations.
    #[serde(default = "d_true")]
    pub animations: bool,
    /// Hover/click/toggle blips.
    #[serde(default = "d_true")]
    pub sounds: bool,
    /// Toast notifications.
    #[serde(default = "d_true")]
    pub notifications: bool,
    /// Extra scale applied to the window (0.75..1.25).
    #[serde(default = "d_ui_scale")]
    pub ui_scale: f32,
    /// Window background opacity (0.55..0.98).
    #[serde(default = "d_opacity")]
    pub opacity: f32,
    /// Draw the ambient glow behind the window.
    #[serde(default = "d_true")]
    pub glow: bool,
    /// Toast anchor: 0 top-right, 1 top-left, 2 bottom-right, 3 bottom-left.
    #[serde(default)]
    pub toast_anchor: usize,

    // --- Friends / waypoints ---
    #[serde(default)]
    pub friends: Vec<String>,
    #[serde(default)]
    pub waypoints: Vec<SavedWaypoint>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SavedWaypoint {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dimension: String,
    #[serde(default = "d_wp_color")]
    pub color: [u8; 4],
}

fn d_wp_color() -> [u8; 4] {
    [124, 92, 255, 255]
}

fn d_win_x() -> f32 {
    20.0
}
fn d_win_y() -> f32 {
    16.0
}
fn d_win_w() -> f32 {
    620.0
}
fn d_win_h() -> f32 {
    360.0
}
fn d_btn_x() -> f32 {
    10.0
}
fn d_btn_y() -> f32 {
    10.0
}

impl Default for ModConfig {
    fn default() -> Self {
        Self {
            menu_open: true,
            window_x: d_win_x(),
            window_y: d_win_y(),
            window_w: d_win_w(),
            window_h: d_win_h(),
            button_x: d_btn_x(),
            button_y: d_btn_y(),
            active_category: 0,
            features_enabled: HashMap::new(),
            float_settings: HashMap::new(),
            bool_settings: HashMap::new(),
            enum_settings: HashMap::new(),
            color_settings: HashMap::new(),
            int_settings: HashMap::new(),
            keybind_settings: HashMap::new(),
            accent: d_accent(),
            theme_index: 0,
            blur: true,
            animations: true,
            sounds: true,
            notifications: true,
            ui_scale: d_ui_scale(),
            opacity: d_opacity(),
            glow: true,
            toast_anchor: 0,
            friends: Vec::new(),
            waypoints: Vec::new(),
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

    /// Saves configuration to disk.
    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

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

    pub fn get_int(&self, key: &str, default_value: i32) -> i32 {
        *self.int_settings.get(key).unwrap_or(&default_value)
    }

    pub fn set_int(&mut self, key: &str, val: i32) {
        self.int_settings.insert(key.to_string(), val);
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

    pub fn get_keybind(&self, key: &str, default_value: &str) -> String {
        self.keybind_settings
            .get(key)
            .cloned()
            .unwrap_or_else(|| default_value.to_string())
    }

    pub fn set_keybind(&mut self, key: &str, val: &str) {
        self.keybind_settings
            .insert(key.to_string(), val.to_string());
    }
}
