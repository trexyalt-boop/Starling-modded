//! Modular feature registration and configuration system.

use super::config::ModConfig;
use serde::{Deserialize, Serialize};

/// Main feature categories as required by the specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeatureCategory {
    Combat,
    Visuals,
    Player,
    World,
    Misc,
    Settings,
}

impl FeatureCategory {
    pub const ALL: [FeatureCategory; 6] = [
        FeatureCategory::Combat,
        FeatureCategory::Visuals,
        FeatureCategory::Player,
        FeatureCategory::World,
        FeatureCategory::Misc,
        FeatureCategory::Settings,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Combat => "Combat",
            Self::Visuals => "Visuals",
            Self::Player => "Player",
            Self::World => "World",
            Self::Misc => "Misc",
            Self::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Combat => "[C]",
            Self::Visuals => "[V]",
            Self::Player => "[P]",
            Self::World => "[W]",
            Self::Misc => "[M]",
            Self::Settings => "[S]",
        }
    }
}

/// Setting value types supported by the mod menu interface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SettingValue {
    Bool(bool),
    Float {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    Enum {
        selected: usize,
        options: Vec<String>,
    },
    Color([u8; 4]),
}

/// A configurable setting within a feature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub name: String,
    pub description: String,
    pub value: SettingValue,
}

impl Setting {
    pub fn bool(key: &str, name: &str, desc: &str, default_val: bool) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            value: SettingValue::Bool(default_val),
        }
    }

    pub fn float(key: &str, name: &str, desc: &str, val: f32, min: f32, max: f32, step: f32) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            value: SettingValue::Float {
                value: val,
                min,
                max,
                step,
            },
        }
    }

    pub fn choice(key: &str, name: &str, desc: &str, selected: usize, options: &[&str]) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            value: SettingValue::Enum {
                selected,
                options: options.iter().map(|s| s.to_string()).collect(),
            },
        }
    }

    pub fn color(key: &str, name: &str, desc: &str, default_color: [u8; 4]) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            value: SettingValue::Color(default_color),
        }
    }
}

/// A registered feature in the Starling mod menu.
#[derive(Clone, Debug)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: FeatureCategory,
    pub enabled: bool,
    pub settings: Vec<Setting>,
    /// Whether settings panel is currently expanded in the UI.
    pub expanded: bool,
}

impl Feature {
    pub fn new(
        id: &str,
        name: &str,
        desc: &str,
        category: FeatureCategory,
        enabled_by_default: bool,
        settings: Vec<Setting>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category,
            enabled: enabled_by_default,
            settings,
            expanded: false,
        }
    }

    pub fn get_bool_setting(&self, key: &str) -> bool {
        for s in &self.settings {
            if s.key == key {
                if let SettingValue::Bool(b) = s.value {
                    return b;
                }
            }
        }
        false
    }

    pub fn get_float_setting(&self, key: &str) -> f32 {
        for s in &self.settings {
            if s.key == key {
                if let SettingValue::Float { value, .. } = s.value {
                    return value;
                }
            }
        }
        0.0
    }

    pub fn get_enum_setting(&self, key: &str) -> usize {
        for s in &self.settings {
            if s.key == key {
                if let SettingValue::Enum { selected, .. } = s.value {
                    return selected;
                }
            }
        }
        0
    }

    pub fn get_color_setting(&self, key: &str) -> [u8; 4] {
        for s in &self.settings {
            if s.key == key {
                if let SettingValue::Color(c) = s.value {
                    return c;
                }
            }
        }
        [255, 255, 255, 255]
    }
}

/// Central registry managing all registered features.
pub struct FeatureRegistry {
    pub features: Vec<Feature>,
}

impl FeatureRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            features: Vec::new(),
        };
        reg.register_defaults();
        reg
    }

    /// Registers the default suite of features according to the specifications.
    fn register_defaults(&mut self) {
        // --- 1. COMBAT MODULES ---
        self.features.push(Feature::new(
            "combat_test",
            "Combat Tester",
            "Controlled simulated attack on targeted entities with rate limiting",
            FeatureCategory::Combat,
            false,
            vec![
                Setting::float("range", "Target Range", "Attack radius in meters", 3.8, 2.0, 6.0, 0.1),
                Setting::float("aps", "Attacks / Sec", "Rate-limited simulated attacks", 8.0, 1.0, 20.0, 1.0),
                Setting::choice("priority", "Target Priority", "Entity selection order", 0, &[
                    "Closest",
                    "Lowest HP",
                    "Crosshair",
                ]),
                Setting::bool("target_monsters", "Target Monsters", "Attack hostile mobs", true),
                Setting::bool("target_players", "Target Players", "Attack other player entities", true),
                Setting::bool("target_passives", "Target Passives", "Attack animals and friendly mobs", false),
                Setting::bool("cooldown_sync", "Weapon Cooldown", "Wait for full 1.9+ weapon charge", true),
            ],
        ));

        self.features.push(Feature::new(
            "silent_aura",
            "Silent Aura",
            "Automatically attacks entities within range silently without client arm animation",
            FeatureCategory::Combat,
            false,
            vec![
                Setting::float("range", "Attack Range", "Reach distance in meters", 3.8, 2.0, 6.0, 0.1),
                Setting::float("aps", "Attacks / Sec", "Rate-limited attacks per second", 8.0, 1.0, 20.0, 1.0),
                Setting::choice("priority", "Target Priority", "Entity selection order", 0, &[
                    "Closest",
                    "Lowest HP",
                    "Crosshair",
                ]),
                Setting::bool("target_monsters", "Target Monsters", "Attack hostile mobs", true),
                Setting::bool("target_players", "Target Players", "Attack other player entities", true),
                Setting::bool("target_passives", "Target Passives", "Attack animals and friendly mobs", false),
                Setting::bool("cooldown_sync", "Weapon Cooldown", "Wait for full 1.9+ weapon charge (0.9+ scale)", true),
                Setting::bool("silent_swing", "Silent Swing", "Suppress local client arm swing animation", true),
            ],
        ));

        self.features.push(Feature::new(
            "trigger_bot",
            "Trigger Bot",
            "Automatically simulates attacks when looking directly at an entity",
            FeatureCategory::Combat,
            false,
            vec![
                Setting::float("delay", "Attack Delay", "Delay after aiming in ms", 50.0, 0.0, 250.0, 10.0),
                Setting::bool("require_weapon", "Require Weapon", "Only trigger when holding a weapon", false),
                Setting::bool("cooldown_sync", "Weapon Cooldown", "Wait for full weapon charge (0.9+ scale)", true),
                Setting::bool("crit_only", "Critical Only", "Only attack when in critical fall state", false),
            ],
        ));

        self.features.push(Feature::new(
            "crit_assist",
            "Critical Helper",
            "Timing indicator and helper for jump-critical attacks",
            FeatureCategory::Combat,
            false,
            vec![
                Setting::bool("hud_indicator", "HUD Indicator", "Visual badge when in critical fall state", true),
                Setting::bool("crit_only", "Critical Only", "Prevent combat attacks unless in crit state", false),
                Setting::bool("auto_jump", "Auto Jump", "Automatically jump in combat to land crits", false),
            ],
        ));

        // --- 2. VISUALS ---
        self.features.push(Feature::new(
            "esp_box",
            "Bounding Box ESP",
            "Draws 2D bounding boxes around living entities",
            FeatureCategory::Visuals,
            true,
            vec![
                Setting::choice("box_style", "Box Style", "Visual outline style", 0, &[
                    "Full Box",
                    "Corners Only",
                ]),
                Setting::float("thickness", "Line Thickness", "Thickness of outlines", 1.5, 1.0, 4.0, 0.5),
                Setting::color("color_monster", "Monster Color", "Box color for hostile mobs", [255, 60, 60, 230]),
                Setting::color("color_player", "Player Color", "Box color for players", [60, 160, 255, 230]),
                Setting::color("color_passive", "Passive Color", "Box color for animals", [60, 240, 100, 230]),
                Setting::bool("show_monsters", "Show Monsters", "Draw boxes on hostile mobs", true),
                Setting::bool("show_players", "Show Players", "Draw boxes on players", true),
                Setting::bool("show_passives", "Show Passives", "Draw boxes on animals", false),
            ],
        ));

        self.features.push(Feature::new(
            "esp_names",
            "Name & Distance ESP",
            "Shows nametags and distance in meters above entities",
            FeatureCategory::Visuals,
            true,
            vec![
                Setting::bool("show_distance", "Show Distance", "Include distance [14m] in label", true),
                Setting::bool("badge_bg", "Background Badge", "Dark background behind nametags", true),
                Setting::float("max_dist", "Max Distance", "Maximum distance to draw labels", 64.0, 16.0, 128.0, 8.0),
            ],
        ));

        self.features.push(Feature::new(
            "esp_health",
            "Health Indicators",
            "Vertical health bar and HP numbers beside entity boxes",
            FeatureCategory::Visuals,
            true,
            vec![
                Setting::bool("health_bar", "Health Bar", "Draw dynamic green-yellow-red bar", true),
                Setting::bool("health_text", "Health Text", "Draw exact HP number (e.g. 18 HP)", true),
            ],
        ));

        self.features.push(Feature::new(
            "target_hud",
            "Target Info HUD",
            "Modern floating HUD showing detailed info on current combat target",
            FeatureCategory::Visuals,
            true,
            vec![
                Setting::float("hud_x", "HUD Position X", "X position in GUI pixels", 10.0, 0.0, 500.0, 10.0),
                Setting::float("hud_y", "HUD Position Y", "Y position in GUI pixels", 50.0, 0.0, 400.0, 10.0),
                Setting::bool("show_hp_bar", "Show HP Bar", "Include animated health bar", true),
            ],
        ));

        self.features.push(Feature::new(
            "tracers",
            "Tracers",
            "Draws lines from bottom of screen to nearby entities",
            FeatureCategory::Visuals,
            false,
            vec![
                Setting::float("max_range", "Max Range", "Max distance for tracers in meters", 48.0, 16.0, 96.0, 4.0),
            ],
        ));

        self.features.push(Feature::new(
            "fullbright",
            "Fullbright",
            "Maximum visual brightness for caves and nighttime",
            FeatureCategory::Visuals,
            false,
            vec![],
        ));

        // --- 3. PLAYER ---
        self.features.push(Feature::new(
            "auto_sprint",
            "Auto Sprint",
            "Automatically engages sprinting whenever walking forward",
            FeatureCategory::Player,
            false,
            vec![],
        ));

        self.features.push(Feature::new(
            "fast_place",
            "Fast Place / Use",
            "Reduces delay for continuous block placing and item interactions",
            FeatureCategory::Player,
            false,
            vec![
                Setting::float("delay_ticks", "Click Delay", "Ticks between actions", 0.0, 0.0, 4.0, 1.0),
            ],
        ));

        self.features.push(Feature::new(
            "step_helper",
            "Step Height",
            "Modifies smooth step height for easier terrain traversal",
            FeatureCategory::Player,
            false,
            vec![
                Setting::float("height", "Step Height", "Step elevation in meters", 1.0, 0.6, 2.0, 0.1),
            ],
        ));

        self.features.push(Feature::new(
            "no_fall_alert",
            "Fall Predictor",
            "Warns when falling from heights that will cause lethal damage",
            FeatureCategory::Player,
            true,
            vec![],
        ));

        // --- 4. WORLD ---
        self.features.push(Feature::new(
            "time_changer",
            "Time of Day Visual",
            "Overrides local day time for visual lighting preview",
            FeatureCategory::World,
            false,
            vec![
                Setting::float("time", "Time of Day", "Tick time: 0=Day, 6000=Noon, 13000=Night", 6000.0, 0.0, 24000.0, 500.0),
            ],
        ));

        self.features.push(Feature::new(
            "weather_clear",
            "Clear Weather Visuals",
            "Hides rain, snow and thunder visual effects locally",
            FeatureCategory::World,
            false,
            vec![],
        ));

        self.features.push(Feature::new(
            "chunk_grid",
            "Chunk Grid",
            "Highlights current chunk boundary coordinates",
            FeatureCategory::World,
            false,
            vec![],
        ));

        // --- 5. MISC ---
        self.features.push(Feature::new(
            "coords_hud",
            "Coordinates HUD",
            "Displays clean on-screen coordinates, facing direction and biome",
            FeatureCategory::Misc,
            true,
            vec![
                Setting::bool("show_biome", "Show Biome", "Include current biome name", true),
                Setting::bool("show_fps", "Show FPS", "Include frame rate display", true),
            ],
        ));

        self.features.push(Feature::new(
            "watermark",
            "Starling Watermark",
            "Shows Starling Mod Menu brand badge in top-left",
            FeatureCategory::Misc,
            true,
            vec![],
        ));

        // --- 6. SETTINGS ---
        self.features.push(Feature::new(
            "menu_scale",
            "Menu GUI Scale",
            "Configures UI sizing factor for high-resolution displays",
            FeatureCategory::Settings,
            false,
            vec![
                Setting::float("scale", "Scale Multiplier", "Scale multiplier", 1.0, 0.7, 1.4, 0.05),
            ],
        ));

        self.features.push(Feature::new(
            "dark_theme",
            "Deep Dark Theme",
            "Toggles between obsidian dark and deep charcoal styling",
            FeatureCategory::Settings,
            true,
            vec![],
        ));

        self.features.push(Feature::new(
            "button_size",
            "Floating Toggle Size",
            "Size of the floating menu toggle button",
            FeatureCategory::Settings,
            false,
            vec![
                Setting::float("size", "Button Size", "Diameter in pixels", 36.0, 24.0, 56.0, 2.0),
            ],
        ));
    }

    /// Synchronizes feature enabled states and setting values from `ModConfig`.
    pub fn load_from_config(&mut self, config: &ModConfig) {
        for f in &mut self.features {
            if let Some(&enabled) = config.features_enabled.get(&f.id) {
                f.enabled = enabled;
            }
            for s in &mut f.settings {
                let key = format!("{}:{}", f.id, s.key);
                match &mut s.value {
                    SettingValue::Bool(b) => {
                        if let Some(&saved) = config.bool_settings.get(&key) {
                            *b = saved;
                        }
                    }
                    SettingValue::Float { value, .. } => {
                        if let Some(&saved) = config.float_settings.get(&key) {
                            *value = saved;
                        }
                    }
                    SettingValue::Enum { selected, .. } => {
                        if let Some(&saved) = config.enum_settings.get(&key) {
                            *selected = saved;
                        }
                    }
                    SettingValue::Color(c) => {
                        if let Some(&saved) = config.color_settings.get(&key) {
                            *c = saved;
                        }
                    }
                }
            }
        }
    }

    /// Saves feature states and setting values to `ModConfig`.
    pub fn save_to_config(&self, config: &mut ModConfig) {
        for f in &self.features {
            config.set_feature_enabled(&f.id, f.enabled);
            for s in &f.settings {
                let key = format!("{}:{}", f.id, s.key);
                match &s.value {
                    SettingValue::Bool(b) => config.set_bool(&key, *b),
                    SettingValue::Float { value, .. } => config.set_float(&key, *value),
                    SettingValue::Enum { selected, .. } => config.set_enum(&key, *selected),
                    SettingValue::Color(c) => config.set_color(&key, *c),
                }
            }
        }
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.features.iter().find(|f| f.id == id).map_or(false, |f| f.enabled)
    }

    pub fn get_feature(&self, id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }

    pub fn get_feature_mut(&mut self, id: &str) -> Option<&mut Feature> {
        self.features.iter_mut().find(|f| f.id == id)
    }

    pub fn toggle_feature(&mut self, id: &str) -> bool {
        if let Some(f) = self.get_feature_mut(id) {
            f.enabled = !f.enabled;
            f.enabled
        } else {
            false
        }
    }
}
