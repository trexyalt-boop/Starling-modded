//! Feature registry and settings for the Corz-style menu.
//!
//! The shape mirrors CorzClient 2.0.1: modules carry a category, a
//! description, a per-module bind and typed settings (boolean, integer,
//! decimal, range, enum, colour and keybind). Ids kept from the Borizon menu
//! still read their old keys from `modmenu-settings.json`.

use super::config::ModConfig;
use serde::{Deserialize, Serialize};

/// The curated categories. CorzClient's server-specific categories
/// (DonutSMP, Bots, Network) are intentionally absent: they need packet and
/// command hooks Horizon does not expose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeatureCategory {
    Combat,
    Visuals,
    Movement,
    World,
    Misc,
    Settings,
}

impl FeatureCategory {
    pub const ALL: [FeatureCategory; 6] = [
        FeatureCategory::Combat,
        FeatureCategory::Visuals,
        FeatureCategory::Movement,
        FeatureCategory::World,
        FeatureCategory::Misc,
        FeatureCategory::Settings,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Combat => "Combat",
            Self::Visuals => "Visuals",
            Self::Movement => "Movement",
            Self::World => "World",
            Self::Misc => "Misc",
            Self::Settings => "Settings",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Combat => 0,
            Self::Visuals => 1,
            Self::Movement => 2,
            Self::World => 3,
            Self::Misc => 4,
            Self::Settings => 5,
        }
    }

    pub fn from_index(i: usize) -> Self {
        Self::ALL[i.min(Self::ALL.len() - 1)]
    }
}

/// A setting value. `Range` is a bounded decimal that the UI draws as a
/// slider with a filled track, matching CorzClient's range rows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SettingValue {
    Bool(bool),
    Float {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    Int {
        value: i32,
        min: i32,
        max: i32,
    },
    Range {
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
    Keybind(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub name: String,
    pub description: String,
    pub value: SettingValue,
}

impl Setting {
    pub fn bool(key: &str, name: &str, desc: &str, default: bool) -> Self {
        Self::new(key, name, desc, SettingValue::Bool(default))
    }

    pub fn float(
        key: &str,
        name: &str,
        desc: &str,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    ) -> Self {
        Self::new(
            key,
            name,
            desc,
            SettingValue::Float {
                value,
                min,
                max,
                step,
            },
        )
    }

    pub fn int(key: &str, name: &str, desc: &str, value: i32, min: i32, max: i32) -> Self {
        Self::new(
            key,
            name,
            desc,
            SettingValue::Int { value, min, max },
        )
    }

    pub fn range(
        key: &str,
        name: &str,
        desc: &str,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    ) -> Self {
        Self::new(
            key,
            name,
            desc,
            SettingValue::Range {
                value,
                min,
                max,
                step,
            },
        )
    }

    pub fn choice(
        key: &str,
        name: &str,
        desc: &str,
        selected: usize,
        options: &[&str],
    ) -> Self {
        Self::new(
            key,
            name,
            desc,
            SettingValue::Enum {
                selected,
                options: options.iter().map(|s| s.to_string()).collect(),
            },
        )
    }

    pub fn color(key: &str, name: &str, desc: &str, rgba: [u8; 4]) -> Self {
        Self::new(key, name, desc, SettingValue::Color(rgba))
    }

    pub fn keybind(key: &str, name: &str, desc: &str, default: &str) -> Self {
        Self::new(
            key,
            name,
            desc,
            SettingValue::Keybind(default.to_string()),
        )
    }

    fn new(key: &str, name: &str, desc: &str, value: SettingValue) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            value,
        }
    }

    pub fn as_bool(&self) -> bool {
        matches!(&self.value, SettingValue::Bool(true))
    }

    pub fn as_float(&self) -> f32 {
        match &self.value {
            SettingValue::Float { value, .. }
            | SettingValue::Range { value, .. } => *value,
            SettingValue::Int { value, .. } => *value as f32,
            _ => 0.0,
        }
    }

    pub fn as_int(&self) -> i32 {
        match &self.value {
            SettingValue::Int { value, .. } => *value,
            SettingValue::Float { value, .. } | SettingValue::Range { value, .. } => {
                value.round() as i32
            }
            _ => 0,
        }
    }

    pub fn as_enum(&self) -> usize {
        match &self.value {
            SettingValue::Enum { selected, .. } => *selected,
            _ => 0,
        }
    }

    pub fn as_color(&self) -> [u8; 4] {
        match &self.value {
            SettingValue::Color(c) => *c,
            _ => [255, 255, 255, 255],
        }
    }

    pub fn as_keybind(&self) -> &str {
        match &self.value {
            SettingValue::Keybind(k) => k,
            _ => "None",
        }
    }

    /// A slider row (float, range or int) exposes (value, min, max, step).
    pub fn slider(&self) -> Option<(f32, f32, f32, f32)> {
        match &self.value {
            SettingValue::Float {
                value,
                min,
                max,
                step,
            }
            | SettingValue::Range {
                value,
                min,
                max,
                step,
            } => Some((*value, *min, *max, *step)),
            SettingValue::Int { value, min, max } => {
                Some((*value as f32, *min as f32, *max as f32, 1.0))
            }
            _ => None,
        }
    }

    pub fn set_slider(&mut self, value: f32) {
        match &mut self.value {
            SettingValue::Float {
                value: v,
                min,
                max,
                step,
            }
            | SettingValue::Range {
                value: v,
                min,
                max,
                step,
            } => {
                let step = step.max(0.0001);
                let snapped = ((value - *min) / step).round() * step + *min;
                *v = snapped.clamp(*min, *max);
            }
            SettingValue::Int {
                value: v,
                min,
                max,
            } => {
                *v = (value.round() as i32).clamp(*min, *max);
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: FeatureCategory,
    pub enabled: bool,
    pub settings: Vec<Setting>,
    /// Whether the settings panel is expanded in the UI.
    pub expanded: bool,
    /// The key that toggles this module ("None" when unbound).
    pub bind: String,
}

impl Feature {
    fn new(
        id: &str,
        name: &str,
        desc: &str,
        category: FeatureCategory,
        settings: Vec<Setting>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category,
            enabled: false,
            settings,
            expanded: false,
            bind: "None".to_string(),
        }
    }

    pub fn setting(&self, key: &str) -> Option<&Setting> {
        self.settings.iter().find(|s| s.key == key)
    }

    pub fn setting_mut(&mut self, key: &str) -> Option<&mut Setting> {
        self.settings.iter_mut().find(|s| s.key == key)
    }

    pub fn get_bool_setting(&self, key: &str) -> bool {
        self.setting(key).is_some_and(|s| s.as_bool())
    }

    pub fn get_float_setting(&self, key: &str) -> f32 {
        self.setting(key).map_or(0.0, |s| s.as_float())
    }

    pub fn get_int_setting(&self, key: &str) -> i32 {
        self.setting(key).map_or(0, |s| s.as_int())
    }

    pub fn get_enum_setting(&self, key: &str) -> usize {
        self.setting(key).map_or(0, |s| s.as_enum())
    }

    pub fn get_color_setting(&self, key: &str) -> [u8; 4] {
        self.setting(key)
            .map_or([255, 255, 255, 255], |s| s.as_color())
    }

    pub fn get_keybind_setting(&self, key: &str) -> &str {
        self.setting(key).map_or("None", |s| s.as_keybind())
    }

    pub fn enum_option(&self, key: &str) -> &str {
        let Some(s) = self.setting(key) else {
            return "";
        };
        if let SettingValue::Enum { selected, options } = &s.value {
            return options.get(*selected).map_or("", |o| o.as_str());
        }
        ""
    }
}

/// Central registry managing all registered features.
pub struct FeatureRegistry {
    pub features: Vec<Feature>,
    /// Saved player names combat modules ignore.
    pub friends: Vec<String>,
}

impl Default for FeatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            features: Vec::new(),
            friends: Vec::new(),
        };
        reg.register_defaults();
        reg
    }

    fn push(&mut self, f: Feature) {
        self.features.push(f);
    }

    fn register_defaults(&mut self) {
        use FeatureCategory::*;
        // ================= COMBAT =================
        self.push(Feature::new(
            "aim_assist",
            "Aim Assist",
            "Gently pulls your aim toward the target nearest your crosshair, in small mouse-like steps.",
            Combat,
            vec![
                Setting::choice("mode", "Mode", "Simple turns at a steady speed; Adaptive slows down as you close in.", 0, &["Simple", "Adaptive"]),
                Setting::choice("targets", "Targets", "What the aim may lock on to.", 0, &["Players", "Players + Mobs", "Mobs"]),
                Setting::bool("require_mouse", "Require Mouse Down", "Only aims while you hold your attack button.", true),
                Setting::bool("aim_vertically", "Aim Vertically", "Also pulls your aim up and down.", false),
                Setting::bool("visibility", "Visibility Check", "Only aims at targets you can see, never through walls.", true),
                Setting::bool("weapons_only", "Weapons Only", "Only aims while you hold a sword, axe, mace, trident or spear.", false),
                Setting::bool("strafe", "Strafe Increase", "Pulls a little harder while you strafe away from the target.", false),
                Setting::float("range", "Aim Range", "Maximum aim distance in meters.", 4.5, 1.0, 8.0, 0.1),
                Setting::float("fov", "Field of View", "Aim cone around the crosshair (360 = all around).", 90.0, 5.0, 360.0, 5.0),
                Setting::float("speed", "Aim Speed", "Fraction of the remaining angle corrected per tick.", 0.35, 0.05, 1.0, 0.05),
            ],
        ));
        self.push(Feature::new(
            "trigger_bot",
            "Trigger Bot",
            "Attacks the target under your crosshair as soon as your weapon is charged.",
            Combat,
            vec![
                Setting::choice("targets", "Targets", "What the bot may attack.", 0, &["Players", "Players + Mobs", "Mobs"]),
                Setting::bool("require_mouse", "Require Mouse Down", "Only attacks while you hold your attack button.", false),
                Setting::bool("require_weapon", "Weapons Only", "Only attack while holding a weapon.", false),
                Setting::bool("ignore_activation", "Ignore Activation Click", "Ignores your first click if it would swing at air.", true),
                Setting::bool("cooldown_sync", "Weapon Cooldown", "Wait for a full weapon charge.", true),
                Setting::int("delay", "Attack Delay", "Minimum time between attacks in milliseconds.", 100, 0, 500),
                Setting::bool("crit_only", "Critical Only", "Only attack while falling.", false),
            ],
        ));
        self.push(Feature::new(
            "auto_clicker",
            "Auto Clicker",
            "Auto-clicks at a configurable CPS, with optional jitter.",
            Combat,
            vec![
                Setting::float("cps", "CPS", "Clicks per second.", 8.0, 1.0, 20.0, 0.5),
                Setting::int("jitter", "Jitter", "Random extra milliseconds between clicks.", 0, 0, 40),
                Setting::bool("only_weapons", "Weapons Only", "Only click while holding a weapon.", false),
                Setting::bool("require_mouse", "Require Mouse Down", "Only click while you hold your attack button.", true),
            ],
        ));
        self.push(Feature::new(
            "crit_assist",
            "Criticals",
            "Times your attacks to land critical hits; optionally jumps for you.",
            Combat,
            vec![
                Setting::bool("hud_indicator", "HUD Indicator", "Show a CRIT READY badge below the crosshair.", true),
                Setting::bool("auto_jump", "Auto Jump", "Jump automatically when your weapon is charged.", false),
                Setting::bool("crit_only", "Critical Only", "Hold attacks until you are falling.", false),
            ],
        ));
        self.push(Feature::new(
            "hit_select",
            "Hit Select",
            "Holds back clicks that would be wasted while your target cannot take damage yet.",
            Combat,
            vec![
                Setting::int("delay", "Extra Delay", "Milliseconds added after the hurt window.", 60, 0, 300),
                Setting::bool("only_players", "Players Only", "Only delay against other players.", true),
            ],
        ));
        self.push(Feature::new(
            "w_tap",
            "W Tap",
            "Releases sprint for a moment after a hit so your next hit deals full knockback.",
            Combat,
            vec![
                Setting::int("hold_ticks", "Release Ticks", "Ticks to stay out of sprint.", 2, 1, 6),
                Setting::bool("only_sprint_hits", "Sprint Hits Only", "Only tap when the hit was a sprint hit.", true),
            ],
        ));
        self.push(Feature::new(
            "auto_jump_reset",
            "Auto Jump Reset",
            "Jumps on the tick an enemy's hit knocks you back so you take less knockback.",
            Combat,
            vec![
                Setting::int("chance", "Chance", "Percent of hits to jump on.", 100, 0, 100),
                Setting::int("delay", "Delay", "Milliseconds to wait before jumping.", 0, 0, 200),
            ],
        ));
        self.push(Feature::new(
            "auto_pot",
            "Auto Pot",
            "Throws an Instant Health splash potion at your feet when your health drops.",
            Combat,
            vec![
                Setting::float("health", "Health", "Throw below this health.", 10.0, 1.0, 20.0, 0.5),
                Setting::choice("potion", "Potion", "Which splash potion to throw.", 0, &["Instant Health", "Regeneration"]),
                Setting::int("delay", "Cooldown", "Milliseconds between throws.", 500, 100, 2000),
                Setting::bool("refill", "Refill Hotbar", "Move more potions into the hotbar when it runs out.", true),
            ],
        ));
        self.push(Feature::new(
            "auto_xp",
            "Auto XP",
            "Throws XP bottles from your hotbar to repair Mending gear.",
            Combat,
            vec![
                Setting::int("durability", "Durability", "Repair when a Mending item falls below this percent.", 60, 5, 100),
                Setting::int("delay", "Cooldown", "Milliseconds between bottles.", 250, 50, 1000),
            ],
        ));
        self.push(Feature::new(
            "key_pearl",
            "Key Pearl",
            "One key swaps to an ender pearl, throws it, then swaps back.",
            Combat,
            vec![
                Setting::keybind("key", "Pearl Key", "Key that throws the pearl.", "G"),
                Setting::int("delay", "Swap Delay", "Milliseconds before swapping back.", 250, 0, 1000),
            ],
        ));
        self.push(Feature::new(
            "auto_mace",
            "Auto Mace",
            "Lands a mace smash for you when you fall onto your target.",
            Combat,
            vec![
                Setting::float("min_fall", "Min Fall", "Minimum fall distance before smashing.", 1.5, 0.5, 5.0, 0.1),
                Setting::bool("only_players", "Players Only", "Only smash other players.", true),
                Setting::bool("stun_slam", "Stun Slam", "Break a raised shield with an axe first.", false),
            ],
        ));
        self.push(Feature::new(
            "trident_assist",
            "Trident Assist",
            "Lands fully charged trident hits and throws or riptides at the right moment.",
            Combat,
            vec![
                Setting::bool("auto_throw", "Auto Throw", "Throw a charged trident at your target.", true),
                Setting::bool("auto_riptide", "Auto Riptide", "Riptide while in water or rain.", false),
                Setting::int("charge", "Charge", "Minimum charge percent before acting.", 100, 50, 100),
            ],
        ));
        self.push(Feature::new(
            "silent_aura",
            "Silent Aura",
            "Attacks entities around you on the server without turning your camera.",
            Combat,
            vec![
                Setting::float("range", "Attack Range", "Reach distance in meters.", 3.8, 2.0, 6.0, 0.1),
                Setting::float("aps", "Attacks / Sec", "Rate-limited attacks per second.", 8.0, 1.0, 20.0, 1.0),
                Setting::float("fov", "Field of View", "Aim cone around the crosshair (360 = all around).", 180.0, 30.0, 360.0, 10.0),
                Setting::choice("priority", "Target Priority", "Entity selection order.", 0, &["Closest", "Lowest HP", "Crosshair"]),
                Setting::bool("target_monsters", "Target Monsters", "Attack hostile mobs.", true),
                Setting::bool("target_players", "Target Players", "Attack other players.", true),
                Setting::bool("target_passives", "Target Passives", "Attack animals and friendly mobs.", false),
                Setting::bool("cooldown_sync", "Weapon Cooldown", "Wait for a full weapon charge.", true),
                Setting::bool("silent_swing", "Silent Swing", "Suppress the local arm swing.", true),
            ],
        ));
        self.push(Feature::new(
            "combat_test",
            "Combat Tester",
            "Controlled, rate-limited attacks for testing in a private world.",
            Combat,
            vec![
                Setting::float("range", "Target Range", "Attack radius in meters.", 3.8, 2.0, 6.0, 0.1),
                Setting::float("aps", "Attacks / Sec", "Rate-limited attacks per second.", 8.0, 1.0, 20.0, 1.0),
                Setting::choice("priority", "Target Priority", "Entity selection order.", 0, &["Closest", "Lowest HP", "Crosshair"]),
                Setting::bool("target_monsters", "Target Monsters", "Attack hostile mobs.", true),
                Setting::bool("target_players", "Target Players", "Attack other players.", true),
                Setting::bool("target_passives", "Target Passives", "Attack animals and friendly mobs.", false),
                Setting::bool("cooldown_sync", "Weapon Cooldown", "Wait for full 1.9+ weapon charge.", true),
            ],
        ));

        // ================= VISUALS =================
        self.push(Feature::new(
            "esp_box",
            "Player ESP",
            "Highlights players, mobs and animals through walls.",
            Visuals,
            vec![
                Setting::choice("box_style", "Style", "Outline or corner brackets.", 0, &["Outline", "Corners"]),
                Setting::float("thickness", "Thickness", "Line thickness in pixels.", 1.5, 1.0, 4.0, 0.5),
                Setting::color("color_player", "Player Color", "Box color for other players.", [90, 160, 255, 230]),
                Setting::color("color_monster", "Monster Color", "Box color for hostile mobs.", [255, 70, 70, 230]),
                Setting::color("color_passive", "Passive Color", "Box color for animals.", [80, 235, 120, 230]),
                Setting::bool("show_players", "Show Players", "Draw other players.", true),
                Setting::bool("show_monsters", "Show Monsters", "Draw hostile mobs.", true),
                Setting::bool("show_passives", "Show Passives", "Draw animals and villagers.", false),
                Setting::float("max_range", "Max Range", "Hide entities farther than this.", 128.0, 16.0, 256.0, 8.0),
                Setting::bool("health_color", "Health Color", "Tint boxes green to red by health.", false),
            ],
        ));
        self.push(Feature::new(
            "esp_names",
            "Name Tags",
            "Draws names and optional health above entities through walls.",
            Visuals,
            vec![
                Setting::bool("show_distance", "Show Distance", "Append the distance to each name.", true),
                Setting::bool("show_health", "Show Health", "Append current health.", false),
                Setting::float("max_dist", "Max Distance", "Hide names farther than this.", 64.0, 8.0, 128.0, 8.0),
            ],
        ));
        self.push(Feature::new(
            "esp_health",
            "Health Bars",
            "Draws a vertical health bar beside each highlighted entity.",
            Visuals,
            vec![Setting::bool("health_bar", "Health Bar", "Show the bar.", true)],
        ));
        self.push(Feature::new(
            "target_hud",
            "Target HUD",
            "A card with your current target's name, health, armor and distance.",
            Visuals,
            vec![
                Setting::float("hud_x", "X", "Card X position.", 10.0, 0.0, 2000.0, 1.0),
                Setting::float("hud_y", "Y", "Card Y position.", 30.0, 0.0, 2000.0, 1.0),
                Setting::bool("show_hp_bar", "Health Bar", "Draw the health bar.", true),
                Setting::bool("show_armor", "Armor", "Draw the target's armor points.", true),
                Setting::bool("show_distance", "Distance", "Draw the distance badge.", true),
            ],
        ));
        self.push(Feature::new(
            "tracers",
            "Tracers",
            "Lines from the bottom of the screen (or the crosshair) to each entity.",
            Visuals,
            vec![
                Setting::float("max_range", "Max Range", "Hide tracers beyond this distance.", 128.0, 16.0, 256.0, 8.0),
                Setting::choice("origin", "Origin", "Where the lines start.", 0, &["Bottom", "Crosshair"]),
                Setting::color("color", "Color", "Tracer color.", [124, 92, 255, 180]),
            ],
        ));
        self.push(Feature::new(
            "fullbright",
            "Fullbright",
            "Brightens the world like night vision.",
            Visuals,
            vec![],
        ));
        self.push(Feature::new(
            "fov_circle",
            "FOV Circle",
            "Draws the combat aim cone around the crosshair.",
            Visuals,
            vec![
                Setting::bool("sync", "Sync to Aura", "Follow the aura's field of view.", true),
                Setting::float("fov", "FOV", "Cone angle when not synced.", 90.0, 5.0, 360.0, 5.0),
                Setting::float("radius", "Radius", "Circle radius in pixels when not synced.", 80.0, 10.0, 400.0, 5.0),
                Setting::float("thickness", "Thickness", "Ring thickness in pixels.", 2.0, 1.0, 6.0, 0.5),
                Setting::color("color", "Color", "Ring color.", [124, 92, 255, 220]),
            ],
        ));
        self.push(Feature::new(
            "coords_hud",
            "Coordinates",
            "Shows your position, heading, biome and dimension.",
            Visuals,
            vec![
                Setting::bool("show_biome", "Biome", "Show the biome name.", true),
                Setting::bool("show_fps", "FPS", "Show the frame rate.", true),
                Setting::bool("show_dimension", "Dimension", "Show the dimension id.", true),
            ],
        ));
        self.push(Feature::new(
            "watermark",
            "Watermark",
            "A small client brand in the corner of the screen.",
            Visuals,
            vec![
                Setting::choice("style", "Style", "Text only or a logo pill.", 0, &["Text", "Minimal"]),
                Setting::bool("show_fps", "FPS", "Append the frame rate.", true),
            ],
        ));
        self.push(Feature::new(
            "module_list",
            "Module List",
            "Lists every enabled module on the HUD, like CorzClient's array list.",
            Visuals,
            vec![
                Setting::float("hud_x", "X", "List X position.", 6.0, 0.0, 2000.0, 1.0),
                Setting::float("hud_y", "Y", "List Y position.", 120.0, 0.0, 2000.0, 1.0),
                Setting::bool("show_binds", "Show Binds", "Append each module's key.", true),
                Setting::choice("sort", "Sort", "Alphabetical or by category.", 0, &["Alphabetical", "Category"]),
            ],
        ));
        self.push(Feature::new(
            "totem_counter",
            "Totem Counter",
            "Shows how many totems you carry, including the offhand.",
            Visuals,
            vec![
                Setting::float("hud_x", "X", "Counter X position.", 6.0, 0.0, 2000.0, 1.0),
                Setting::float("hud_y", "Y", "Counter Y position.", 200.0, 0.0, 2000.0, 1.0),
                Setting::bool("show_offhand", "Offhand", "Include the offhand totem.", true),
            ],
        ));
        self.push(Feature::new(
            "waypoints",
            "Waypoints",
            "Mark spots in the world and see them as a beam, icon and distance label.",
            Visuals,
            vec![
                Setting::color("color", "Color", "Beam and label color.", [124, 92, 255, 220]),
                Setting::float("max_dist", "Max Distance", "Hide waypoints beyond this.", 512.0, 32.0, 2048.0, 16.0),
                Setting::bool("beam", "Beam", "Draw a vertical beam.", true),
                Setting::bool("label", "Label", "Draw the name and distance.", true),
                Setting::keybind("add_key", "Add Key", "Add a waypoint where you stand.", "B"),
            ],
        ));
        self.push(Feature::new(
            "trails",
            "Trails",
            "A glowing ribbon that follows you through the world.",
            Visuals,
            vec![
                Setting::int("length", "Length", "Number of recorded points.", 24, 4, 64),
                Setting::color("color", "Color", "Trail color.", [124, 92, 255, 200]),
                Setting::bool("only_self", "Only Self", "Do not draw other players' trails.", true),
                Setting::float("width", "Width", "Ribbon width in pixels.", 2.0, 1.0, 6.0, 0.5),
            ],
        ));
        self.push(Feature::new(
            "jump_circles",
            "Jump Circles",
            "Stamps a glowing ring around you when you jump or land.",
            Visuals,
            vec![
                Setting::choice("style", "Style", "Ring or a ripple of blocks.", 0, &["Ring", "Ripple"]),
                Setting::color("color", "Color", "Ring color.", [124, 92, 255, 220]),
                Setting::float("size", "Size", "Ring radius in blocks.", 1.0, 0.5, 3.0, 0.1),
            ],
        ));
        self.push(Feature::new(
            "storage_esp",
            "Storage ESP",
            "Highlights chests, barrels, shulkers and spawners through walls.",
            Visuals,
            vec![
                Setting::color("color", "Color", "Box color.", [255, 200, 60, 220]),
                Setting::float("max_dist", "Max Distance", "Hide storage beyond this.", 96.0, 16.0, 256.0, 8.0),
                Setting::bool("chests", "Chests", "Draw chests and barrels.", true),
                Setting::bool("spawners", "Spawners", "Draw mob spawners.", true),
                Setting::bool("other", "Other", "Draw furnaces, hoppers and other containers.", false),
            ],
        ));
        self.push(Feature::new(
            "custom_crosshair",
            "Custom Crosshair",
            "Draws a clean custom crosshair and hides the vanilla look.",
            Visuals,
            vec![
                Setting::choice("style", "Style", "Cross, dot or circle.", 0, &["Cross", "Dot", "Circle"]),
                Setting::color("color", "Color", "Crosshair color.", [255, 255, 255, 230]),
                Setting::int("size", "Size", "Arm length in pixels.", 6, 2, 20),
                Setting::int("gap", "Gap", "Centre gap in pixels.", 3, 0, 10),
                Setting::bool("dynamic", "Dynamic", "Expand the gap while attacking.", true),
            ],
        ));
        self.push(Feature::new(
            "time_changer",
            "Time Changer",
            "Changes the client-side time of day.",
            Visuals,
            vec![Setting::int("time", "Time", "Ticks (0 = sunrise, 6000 = noon, 18000 = midnight).", 6000, 0, 24000)],
        ));
        self.push(Feature::new(
            "weather_changer",
            "Weather Changer",
            "Forces the client-side weather.",
            Visuals,
            vec![Setting::choice("weather", "Weather", "Clear, rain or thunder.", 0, &["Clear", "Rain", "Thunder"])],
        ));
        self.push(Feature::new(
            "zoom",
            "Zoom",
            "Optical zoom on a key; the setting picks the factor.",
            Visuals,
            vec![
                Setting::float("factor", "Factor", "Zoom multiplier.", 4.0, 2.0, 10.0, 0.5),
                Setting::bool("smooth", "Smooth", "Ease in and out.", true),
                Setting::keybind("key", "Zoom Key", "Hold or toggle the zoom.", "C"),
            ],
        ));
        self.push(Feature::new(
            "block_highlight",
            "Block Highlight",
            "Highlights the block you are looking at.",
            Visuals,
            vec![
                Setting::color("color", "Color", "Outline color.", [124, 92, 255, 230]),
                Setting::float("thickness", "Thickness", "Outline thickness.", 1.5, 1.0, 4.0, 0.5),
                Setting::int("fill_alpha", "Fill Alpha", "Translucent fill strength (0 = off).", 40, 0, 120),
            ],
        ));

        // ================= MOVEMENT =================
        self.push(Feature::new(
            "auto_sprint",
            "Sprint",
            "Keeps you sprinting automatically while moving forward.",
            Movement,
            vec![
                Setting::choice("mode", "Mode", "Always or only while moving forward.", 0, &["Always", "Forward"]),
                Setting::bool("require_food", "Require Food", "Do not sprint when you cannot.", true),
            ],
        ));
        self.push(Feature::new(
            "keep_sprint",
            "Keep Sprint",
            "Keeps you sprinting through hits.",
            Movement,
            vec![Setting::bool("only_hits", "Only Hits", "Only re-sprint after a hit.", true)],
        ));
        self.push(Feature::new(
            "auto_walk",
            "Auto Walk",
            "Walks forward automatically.",
            Movement,
            vec![
                Setting::float("speed", "Speed", "Fraction of forward input.", 1.0, 0.2, 1.0, 0.1),
                Setting::bool("jump", "Auto Jump", "Jump when something blocks the way.", false),
            ],
        ));
        self.push(Feature::new(
            "step_helper",
            "Step Assist",
            "Raises your step height so you walk over low blocks.",
            Movement,
            vec![Setting::float("height", "Height", "Step height in blocks.", 1.0, 0.5, 1.5, 0.1)],
        ));

        // ================= WORLD =================
        self.push(Feature::new(
            "fast_place",
            "Fast Place",
            "Removes the block-placement delay.",
            World,
            vec![Setting::int("delay_ticks", "Delay", "Remaining delay in ticks.", 0, 0, 4)],
        ));
        self.push(Feature::new(
            "fastbreak",
            "Fast Break",
            "Shortens the block-break delay.",
            World,
            vec![
                Setting::float("multiplier", "Multiplier", "Break speed multiplier.", 1.3, 1.0, 3.0, 0.1),
                Setting::bool("only_blocks", "Blocks Only", "Do not speed up entities.", true),
            ],
        ));
        self.push(Feature::new(
            "inventory_cleaner",
            "Inventory Cleaner",
            "Throws junk and worse copies of your gear out of the inventory.",
            World,
            vec![
                Setting::bool("keep_tools", "Keep Tools", "Never drop tools or weapons.", true),
                Setting::bool("keep_food", "Keep Food", "Never drop food.", true),
                Setting::bool("keep_blocks", "Keep Blocks", "Never drop building blocks.", true),
                Setting::int("threshold", "Stack Size", "Drop stacks smaller than this.", 2, 1, 8),
            ],
        ));
        self.push(Feature::new(
            "auto_tool",
            "Auto Tool",
            "Selects the fastest correct tool for the block you are breaking.",
            World,
            vec![
                Setting::choice("mode", "Mode", "Which tool to prefer.", 0, &["Speed", "Silk Touch", "Fortune"]),
                Setting::bool("swap_back", "Swap Back", "Return to your previous slot when the block is gone.", true),
            ],
        ));

        // ================= MISC =================
        self.push(Feature::new(
            "fake_coords",
            "Fake Coordinates",
            "Shows offset coordinates on the HUD so clips never leak where you are.",
            Misc,
            vec![
                Setting::float("offset_x", "Offset X", "Added to the displayed X.", 0.0, -1000.0, 1000.0, 1.0),
                Setting::float("offset_y", "Offset Y", "Added to the displayed Y.", 0.0, -256.0, 256.0, 1.0),
                Setting::float("offset_z", "Offset Z", "Added to the displayed Z.", 0.0, -1000.0, 1000.0, 1.0),
            ],
        ));
        self.push(Feature::new(
            "quick_friend",
            "Quick Friend",
            "One key adds or removes the player you are looking at from your friends.",
            Misc,
            vec![Setting::keybind("key", "Friend Key", "Add or remove the player under your crosshair.", "V")],
        ));
        self.push(Feature::new(
            "friends",
            "Friends",
            "Combat modules ignore saved players. Tap a name to remove it.",
            Misc,
            vec![],
        ));

        // ================= SETTINGS =================
        self.push(Feature::new(
            "gui_settings",
            "GUI Settings",
            "Accent colour, blur, animations, opacity and toast position.",
            Settings,
            vec![
                Setting::color("accent", "Accent", "The interface accent colour.", [124, 92, 255, 255]),
                Setting::bool("blur", "Frosted Blur", "Blur the world behind the window.", true),
                Setting::bool("animations", "Animations", "Open, hover and category transitions.", true),
                Setting::bool("glow", "Ambient Glow", "Animated glow behind the window.", true),
                Setting::float("opacity", "Opacity", "Window background opacity.", 0.86, 0.55, 0.98, 0.01),
                Setting::choice("toast_anchor", "Toasts", "Where notifications appear.", 0, &["Top Right", "Top Left", "Bottom Right", "Bottom Left"]),
            ],
        ));
        self.push(Feature::new(
            "client_sounds",
            "Client Sounds",
            "Plays a blip when you hover, click or toggle a module.",
            Settings,
            vec![
                Setting::float("volume", "Volume", "Sound volume.", 0.6, 0.0, 1.0, 0.05),
                Setting::bool("hover", "Hover Sound", "Blip when the pointer enters a card.", true),
                Setting::bool("toggle", "Toggle Sound", "Blip when a module turns on or off.", true),
            ],
        ));
        self.push(Feature::new(
            "notifications",
            "Notifications",
            "Toast notifications in the corner of the screen.",
            Settings,
            vec![
                Setting::bool("toggles", "Module Toasts", "Toast when a module toggles.", true),
                Setting::bool("join", "Welcome Toast", "Toast when the menu first opens.", false),
            ],
        ));
        self.push(Feature::new(
            "panic",
            "Panic",
            "Instantly disables every active module.",
            Settings,
            vec![Setting::keybind("key", "Panic Key", "Disable everything.", "None")],
        ));
        self.push(Feature::new(
            "configs",
            "Configs",
            "Save, reload or reset the client configuration.",
            Settings,
            vec![],
        ));
    }

    pub fn load_from_config(&mut self, config: &ModConfig) {
        self.friends = config.friends.clone();
        for f in &mut self.features {
            f.enabled = config.is_feature_enabled(&f.id, false);
            f.bind = config.get_keybind(&format!("feature:{}", f.id), "None");
            for s in &mut f.settings {
                let key = format!("{}:{}", f.id, s.key);
                match &mut s.value {
                    SettingValue::Bool(v) => *v = config.get_bool(&key, *v),
                    SettingValue::Float { value, .. } | SettingValue::Range { value, .. } => {
                        *value = config.get_float(&key, *value)
                    }
                    SettingValue::Int { value, .. } => *value = config.get_int(&key, *value),
                    SettingValue::Enum { selected, .. } => {
                        *selected = config.get_enum(&key, *selected)
                    }
                    SettingValue::Color(v) => *v = config.get_color(&key, *v),
                    SettingValue::Keybind(v) => *v = config.get_keybind(&key, v),
                }
            }
        }
        // The GUI settings own the appearance fields.
        if let Some(gui) = self.get_feature_mut("gui_settings") {
            if let Some(Setting {
                value: SettingValue::Color(accent),
                ..
            }) = gui.setting_mut("accent")
            {
                *accent = config.accent;
            }
            if let Some(Setting {
                value: SettingValue::Bool(blur),
                ..
            }) = gui.setting_mut("blur")
            {
                *blur = config.blur;
            }
            if let Some(Setting {
                value: SettingValue::Bool(animations),
                ..
            }) = gui.setting_mut("animations")
            {
                *animations = config.animations;
            }
            if let Some(Setting {
                value: SettingValue::Bool(glow),
                ..
            }) = gui.setting_mut("glow")
            {
                *glow = config.glow;
            }
            if let Some(Setting {
                value: SettingValue::Float { value, .. },
                ..
            }) = gui.setting_mut("opacity")
            {
                *value = config.opacity;
            }
            if let Some(Setting {
                value: SettingValue::Enum { selected, .. },
                ..
            }) = gui.setting_mut("toast_anchor")
            {
                *selected = config.toast_anchor.min(3);
            }
        }
    }

    pub fn save_to_config(&self, config: &mut ModConfig) {
        config.friends = self.friends.clone();
        if let Some(gui) = self.get_feature("gui_settings") {
            config.accent = gui.get_color_setting("accent");
            config.blur = gui.get_bool_setting("blur");
            config.animations = gui.get_bool_setting("animations");
            config.glow = gui.get_bool_setting("glow");
            config.opacity = gui.get_float_setting("opacity").clamp(0.55, 0.98);
            config.toast_anchor = gui.get_enum_setting("toast_anchor").min(3);
        }
        for f in &self.features {
            config.set_feature_enabled(&f.id, f.enabled);
            config.set_keybind(&format!("feature:{}", f.id), &f.bind);
            for s in &f.settings {
                let key = format!("{}:{}", f.id, s.key);
                match &s.value {
                    SettingValue::Bool(v) => config.set_bool(&key, *v),
                    SettingValue::Float { value, .. } | SettingValue::Range { value, .. } => {
                        config.set_float(&key, *value)
                    }
                    SettingValue::Int { value, .. } => config.set_int(&key, *value),
                    SettingValue::Enum { selected, .. } => config.set_enum(&key, *selected),
                    SettingValue::Color(v) => config.set_color(&key, *v),
                    SettingValue::Keybind(v) => config.set_keybind(&key, v),
                }
            }
        }
    }

    pub fn get_feature(&self, id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }

    pub fn get_feature_mut(&mut self, id: &str) -> Option<&mut Feature> {
        self.features.iter_mut().find(|f| f.id == id)
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.get_feature(id).is_some_and(|f| f.enabled)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(f) = self.get_feature_mut(id) {
            f.enabled = enabled;
        }
    }

    pub fn toggle_feature(&mut self, id: &str) {
        if let Some(f) = self.get_feature_mut(id) {
            f.enabled = !f.enabled;
        }
    }

    /// The feature whose module bind is `key` (a winit key name).
    pub fn feature_for_bind(&self, key: &str) -> Option<&str> {
        if key == "None" {
            return None;
        }
        self.features
            .iter()
            .find(|f| f.bind == key)
            .map(|f| f.id.as_str())
    }

    /// A feature whose own keybind setting matches `key`.
    pub fn feature_for_setting_key(&self, key: &str) -> Option<&str> {
        self.features
            .iter()
            .find(|f| {
                f.settings.iter().any(|s| {
                    matches!(&s.value, SettingValue::Keybind(k) if k == key && key != "None")
                })
            })
            .map(|f| f.id.as_str())
    }

    /// Every enabled module, in display order.
    pub fn enabled(&self) -> impl Iterator<Item = &Feature> {
        self.features.iter().filter(|f| f.enabled)
    }

    /// Disables everything and returns how many modules were on.
    pub fn panic(&mut self) -> usize {
        let mut n = 0;
        for f in &mut self.features {
            if f.enabled {
                f.enabled = false;
                n += 1;
            }
        }
        n
    }

    pub fn reset_settings(&mut self) {
        for f in &mut self.features {
            f.enabled = false;
            f.expanded = false;
            f.bind = "None".to_string();
        }
    }

    /// Reads the colour a feature uses for boxes, so several renderers agree.
    pub fn color_of(&self, id: &str, key: &str, default: [u8; 4]) -> [u8; 4] {
        self.get_feature(id)
            .map_or(default, |f| f.get_color_setting(key))
    }

    pub fn is_friend(&self, name: &str) -> bool {
        self.friends
            .iter()
            .any(|f| f.eq_ignore_ascii_case(name))
    }

    /// Adds `name`; returns false when it was already a friend.
    pub fn add_friend(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || self.is_friend(name) {
            return false;
        }
        self.friends.push(name.to_string());
        true
    }

    pub fn remove_friend(&mut self, name: &str) {
        self.friends.retain(|f| !f.eq_ignore_ascii_case(name));
    }

    pub fn bind_label(&self, id: &str) -> Option<&str> {
        self.get_feature(id)
            .map(|f| f.bind.as_str())
            .filter(|b| *b != "None")
    }
}
