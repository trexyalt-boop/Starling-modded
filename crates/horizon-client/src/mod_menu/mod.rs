//! Starling's Corz-style mod menu.
//!
//! One `ModMenu` owns the feature registry, the persistent configuration, the
//! ClickGUI state, the combat bookkeeping and the toasts. The game calls
//! `tick` once per game tick, `keyboard`/`touch_event` for input, and
//! `render_visuals`/`render_menu` while building the HUD batch.

pub mod combat;
pub mod config;
pub mod feature;
pub mod notifications;
pub mod theme;
pub mod ui;
pub mod visuals;

pub use combat::CombatSystem;
pub use config::{ModConfig, SavedWaypoint};
pub use feature::{Feature, FeatureCategory, FeatureRegistry, Setting, SettingValue};
pub use notifications::Notifications;
pub use theme::Palette;
pub use ui::{SoundKind, UiAction, UiState};
pub use visuals::HudData;

use horizon_client_render::hud::{HudAtlas, HudBatch};
use winit::event::TouchPhase;
use winit::keyboard::{KeyCode, PhysicalKey};

/// Core Starling Mod Menu instance embedded in the game client.
pub struct ModMenu {
    pub config: ModConfig,
    pub registry: FeatureRegistry,
    pub ui_state: UiState,
    pub combat: CombatSystem,
    pub notifications: Notifications,
    pub palette: Palette,

    /// One-shot requests raised by a key and consumed by the game.
    pub pearl_requested: bool,
    pub waypoint_requested: bool,
    pub quick_friend_requested: bool,
    /// A menu action the game must apply (needs the world).
    pub pending_external_action: Option<UiAction>,
}

impl Default for ModMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl ModMenu {
    /// Creates the menu, loading persistent settings from disk.
    pub fn new() -> Self {
        let config = ModConfig::load();
        let mut registry = FeatureRegistry::new();
        registry.load_from_config(&config);
        let ui_state = UiState::new(&config);
        let palette = theme::palette(config.accent, config.opacity);
        Self {
            config,
            registry,
            ui_state,
            combat: CombatSystem::new(),
            notifications: Notifications::default(),
            palette,
            pearl_requested: false,
            waypoint_requested: false,
            quick_friend_requested: false,
            pending_external_action: None,
        }
    }

    // ---------------------------------------------------------------- state

    pub fn is_open(&self) -> bool {
        self.ui_state.open
    }

    pub fn set_open(&mut self, open: bool) {
        if self.ui_state.open == open {
            return;
        }
        self.ui_state.open = open;
        self.ui_state.save_to_config(&mut self.config);
        self.config.save();
    }

    pub fn toggle_open(&mut self) {
        self.set_open(!self.ui_state.open);
    }

    /// Frosted blur behind the window, when the GUI setting asks for it.
    pub fn gui_blur(&self) -> bool {
        self.ui_state.open
            && self
                .registry
                .get_feature("gui_settings")
                .is_none_or(|f| f.get_bool_setting("blur"))
    }

    pub fn animations(&self) -> bool {
        self.config.animations
    }

    pub fn sounds_enabled(&self) -> bool {
        self.config.sounds
    }

    pub fn sound_volume(&self) -> f32 {
        self.registry
            .get_feature("client_sounds")
            .map_or(0.6, |f| f.get_float_setting("volume"))
    }

    pub fn toast_anchor(&self) -> usize {
        self.registry
            .get_feature("gui_settings")
            .map_or(self.config.toast_anchor, |f| {
                f.get_enum_setting("toast_anchor")
            })
            .min(3)
    }

    /// Whether any HUD/ESP renderer has work to do this frame.
    pub fn visuals_active(&self) -> bool {
        const IDS: [&str; 19] = [
            "esp_box",
            "esp_names",
            "esp_health",
            "target_hud",
            "tracers",
            "coords_hud",
            "fov_circle",
            "watermark",
            "module_list",
            "totem_counter",
            "waypoints",
            "trails",
            "jump_circles",
            "storage_esp",
            "custom_crosshair",
            "block_highlight",
            "crit_assist",
            "time_changer",
            "fullbright",
        ];
        IDS.iter().any(|id| self.registry.is_enabled(id))
    }

    // ---------------------------------------------------------------- tick

    /// Advances animations and notifications, and mirrors the accent into the
    /// palette. Called once per game tick.
    pub fn tick(&mut self, dt: f32) {
        // The GUI settings own the accent; pick up a change made in the menu
        // or a preset tap.
        let accent = self
            .registry
            .get_feature("gui_settings")
            .map_or(self.config.accent, |f| f.get_color_setting("accent"));
        let opacity = self
            .registry
            .get_feature("gui_settings")
            .map_or(self.config.opacity, |f| f.get_float_setting("opacity"));
        if accent != self.config.accent || (opacity - self.config.opacity).abs() > 0.001 {
            self.config.accent = accent;
            self.config.opacity = opacity;
            self.palette = theme::palette(accent, opacity);
        }
        self.ui_state.set_friends_cache(self.config.friends.clone());
        self.ui_state.set_accent_cache(self.config.accent);
        self.ui_state
            .tick(dt, self.registry.get_feature("gui_settings").map_or(true, |f| {
                f.get_bool_setting("animations")
            }));
        self.notifications.tick(dt);

        // Toasts raised by the UI become notifications.
        if let Some((title, message)) = self.ui_state.take_toast() {
            if self.config.notifications {
                let accent = self.palette.accent;
                self.notifications.push(&title, &message, accent);
            }
        }
        if let Some(action) = self.ui_state.take_action() {
            if !self.apply_action(action.clone()) {
                // Handled by the game (friends and waypoints need the world).
                self.pending_external_action = Some(action);
            }
        }
    }

    /// An action the game must resolve (adding a friend, adding a waypoint).
    pub fn take_external_action(&mut self) -> Option<UiAction> {
        self.pending_external_action.take()
    }

    /// Handles the menu's own actions. Returns false for actions that need
    /// the game (adding a friend, adding a waypoint).
    pub fn apply_action(&mut self, action: UiAction) -> bool {
        match action {
            UiAction::SaveConfig => {
                self.save();
                self.toast("Config", "Saved");
            }
            UiAction::ReloadConfig => {
                self.reload();
                self.toast("Config", "Reloaded");
            }
            UiAction::ResetConfig => {
                self.reset();
                self.toast("Config", "Reset to defaults");
            }
            UiAction::Panic => {
                let n = self.registry.panic();
                self.save();
                self.toast("Panic", &format!("{n} modules disabled"));
            }
            UiAction::RemoveFriend(name) => {
                self.registry.remove_friend(&name);
                self.config.friends = self.registry.friends.clone();
                self.config.save();
                self.toast("Friends", &format!("Removed {name}"));
            }
            UiAction::SetAccent(color) => {
                if let Some(f) = self.registry.get_feature_mut("gui_settings") {
                    if let Some(s) = f.setting_mut("accent") {
                        s.value = SettingValue::Color(color);
                    }
                }
                self.config.accent = color;
                self.palette = theme::palette(color, self.config.opacity);
                self.config.save();
            }
            UiAction::AddFriend | UiAction::AddWaypoint => return false,
        }
        true
    }

    pub fn save(&mut self) {
        self.registry.save_to_config(&mut self.config);
        self.ui_state.save_to_config(&mut self.config);
        self.config.friends = self.registry.friends.clone();
        self.config.save();
    }

    pub fn reload(&mut self) {
        self.config = ModConfig::load();
        self.registry.load_from_config(&self.config);
        self.ui_state = UiState::new(&self.config);
        self.palette = theme::palette(self.config.accent, self.config.opacity);
    }

    pub fn reset(&mut self) {
        let accent = self.config.accent;
        self.config = ModConfig::default();
        self.config.accent = accent;
        self.registry = FeatureRegistry::new();
        self.registry.friends = self.config.friends.clone();
        self.ui_state = UiState::new(&self.config);
        self.palette = theme::palette(self.config.accent, self.config.opacity);
        self.save();
    }

    pub fn toast(&mut self, title: &str, message: &str) {
        if !self.config.notifications {
            return;
        }
        let accent = self.palette.accent;
        self.notifications.push(title, message, accent);
    }

    pub fn add_friend(&mut self, name: &str) {
        if self.registry.add_friend(name) {
            self.config.friends = self.registry.friends.clone();
            self.config.save();
            self.toast("Friends", &format!("Added {name}"));
        }
    }

    pub fn add_waypoint(&mut self, name: &str, pos: [f64; 3], dimension: &str) {
        let color = self
            .registry
            .get_feature("waypoints")
            .map_or([124, 92, 255, 220], |f| f.get_color_setting("color"));
        self.config.waypoints.push(SavedWaypoint {
            name: name.to_string(),
            x: pos[0],
            y: pos[1],
            z: pos[2],
            dimension: dimension.to_string(),
            color,
        });
        self.config.save();
        self.toast("Waypoint", name);
    }

    // ---------------------------------------------------------------- input

    /// A keyboard event. `name` is the canonical `key_name` of the physical
    /// key ("key.keyboard.c"), `text` its typed character. Returns true when
    /// the menu consumed the event.
    pub fn keyboard(
        &mut self,
        name: Option<&str>,
        text: Option<&str>,
        pressed: bool,
        repeat: bool,
    ) -> bool {
        let Some(name) = name else {
            return self.ui_state.open && self.ui_state.search_focus;
        };

        // Capture mode first: the next key becomes a bind.
        if self.ui_state.open && self.ui_state.listening.is_some() {
            if pressed && !repeat {
                let short = ui::pretty_key(name);
                self.ui_state.key_event(&short, &mut self.registry);
            }
            return true;
        }

        if self.ui_state.open && self.ui_state.search_focus {
            if pressed && !repeat {
                match name {
                    "key.keyboard.escape" => {
                        self.ui_state.search_focus = false;
                    }
                    "key.keyboard.backspace" => {
                        self.ui_state.backspace();
                    }
                    "key.keyboard.enter" | "key.keyboard.numpad.enter" => {
                        self.ui_state.search_focus = false;
                    }
                    _ => {
                        if let Some(text) = text {
                            for ch in text.chars() {
                                self.ui_state.text_input(ch);
                            }
                        }
                    }
                }
            }
            return true;
        }

        if self.ui_state.open && name == "key.keyboard.escape" && pressed && !repeat {
            self.set_open(false);
            return true;
        }

        // A module's own bind, a panic key, or one of the per-module keys.
        let short = ui::pretty_key(name);
        if pressed && !repeat {
            let mut toggled: Option<(String, String, bool)> = None;
            if let Some(id) = self.registry.feature_for_bind(&short).map(str::to_string) {
                if let Some(f) = self.registry.get_feature_mut(&id) {
                    f.enabled = !f.enabled;
                    toggled = Some((id, f.name.clone(), f.enabled));
                }
            }
            if let Some((_, name, on)) = toggled {
                self.save();
                self.toast(
                    if on { "Enabled" } else { "Disabled" },
                    &name,
                );
                return true;
            }
            if self.panic_pressed(&short) {
                let n = self.registry.panic();
                self.save();
                self.toast("Panic", &format!("{n} modules disabled"));
                return true;
            }
        }

        // Zoom: the key toggles, and stays up until pressed again.
        if pressed && !repeat && self.key_for("zoom", "key", &short) {
            if self.registry.is_enabled("zoom") {
                self.combat.zoom = !self.combat.zoom;
                return true;
            }
        }
        if pressed && !repeat {
            if self.key_for("key_pearl", "key", &short) && self.registry.is_enabled("key_pearl") {
                self.pearl_requested = true;
                return true;
            }
            if self.key_for("waypoints", "add_key", &short)
                && self.registry.is_enabled("waypoints")
            {
                self.waypoint_requested = true;
                return true;
            }
            if self.key_for("quick_friend", "key", &short)
                && self.registry.is_enabled("quick_friend")
            {
                self.quick_friend_requested = true;
                return true;
            }
        }

        // While the menu is open it owns the keyboard, so typing never leaks
        // into the world.
        self.ui_state.open
    }

    /// The old gameplay shortcut path: a module bind or the panic key.
    pub fn handle_shortcut(&mut self, key: KeyCode, vanilla_bound: bool) -> bool {
        let Some(name) = horizon_client::keybinds::key_name(key) else {
            return false;
        };
        if vanilla_bound {
            return false;
        }
        let short = ui::pretty_key(name);
        if let Some(id) = self.registry.feature_for_bind(&short).map(str::to_string) {
            self.registry.toggle_feature(&id);
            let (name, on) = self
                .registry
                .get_feature(&id)
                .map(|f| (f.name.clone(), f.enabled))
                .unwrap_or_default();
            self.save();
            self.toast(if on { "Enabled" } else { "Disabled" }, &name);
            return true;
        }
        if self.panic_pressed(&short) {
            let n = self.registry.panic();
            self.save();
            self.toast("Panic", &format!("{n} modules disabled"));
            return true;
        }
        false
    }

    fn panic_pressed(&self, short: &str) -> bool {
        self.registry
            .get_feature("panic")
            .is_some_and(|f| ui::pretty_key(f.get_keybind_setting("key")) == short)
    }

    fn key_for(&self, feature: &str, setting: &str, short: &str) -> bool {
        self.registry.get_feature(feature).is_some_and(|f| {
            let key = f.get_keybind_setting(setting);
            key != "None" && ui::pretty_key(key) == short
        })
    }

    /// Android touch, in physical pixels. `scale` is the GUI scale of the
    /// batch the menu is drawn into.
    pub fn touch_event(
        &mut self,
        phase: TouchPhase,
        point: [f32; 2],
        touch_id: u64,
        scale: f32,
        atlas: &HudAtlas,
    ) -> bool {
        let s = scale.max(1.0);
        let gui = [point[0] / s, point[1] / s];
        let friends = self.config.friends.clone();
        let consumed = ui::handle_touch(
            phase,
            gui,
            touch_id,
            &mut self.ui_state,
            &mut self.registry,
            &mut self.config,
            &self.palette,
            &friends,
            atlas,
        );
        if consumed {
            self.ui_state.set_friends_cache(self.config.friends.clone());
        }
        consumed
    }

    // ---------------------------------------------------------------- render

    /// Draws every enabled world/HUD visual.
    pub fn render_visuals(&mut self, batch: &mut HudBatch, atlas: &HudAtlas, data: &HudData) {
        visuals::render(batch, atlas, &self.registry, data);
    }

    /// Draws the floating button, the window and the toasts.
    pub fn render_menu(&mut self, batch: &mut HudBatch, atlas: &HudAtlas) {
        let animations = self.animations();
        ui::draw_floating_button(batch, atlas, &self.ui_state, &self.palette);
        ui::draw_menu(
            batch,
            atlas,
            &mut self.ui_state,
            &self.registry,
            &self.palette,
            animations,
        );
        ui::draw_notifications(
            batch,
            atlas,
            &self.notifications,
            &self.palette,
            self.toast_anchor(),
            self.animations(),
        );
    }

    /// The physical key of a `KeyCode`, for the game's key dispatch.
    pub fn key_name(code: KeyCode) -> Option<&'static str> {
        horizon_client::keybinds::key_name(code)
    }

    /// The name of a physical key event.
    pub fn physical_name(key: PhysicalKey) -> Option<&'static str> {
        match key {
            PhysicalKey::Code(code) => Self::key_name(code),
            PhysicalKey::Unidentified(_) => None,
        }
    }
}

pub fn combat_is_passive(kind: &str) -> bool {
    let k = kind.trim_start_matches("minecraft:");
    matches!(
        k,
        "cow"
            | "pig"
            | "sheep"
            | "chicken"
            | "villager"
            | "horse"
            | "donkey"
            | "mule"
            | "cat"
            | "wolf"
            | "parrot"
            | "rabbit"
            | "bat"
            | "squid"
            | "glow_squid"
            | "dolphin"
            | "turtle"
            | "axolotl"
            | "allay"
            | "frog"
            | "tadpole"
            | "camel"
            | "sniffer"
            | "armadillo"
            | "iron_golem"
            | "snow_golem"
    )
}
