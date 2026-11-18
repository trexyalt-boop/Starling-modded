//! Starling Android Native Mod Menu
//!
//! Fully integrated native Rust mod menu providing modern dark-themed floating UI,
//! touchscreen support, persistent settings, ESP visualization, and modular combat testing.

pub mod combat;
pub mod config;
pub mod feature;
pub mod ui;
pub mod visuals;

pub use combat::CombatSystem;
pub use config::ModConfig;
pub use feature::{Feature, FeatureCategory, FeatureRegistry, Setting, SettingValue};
pub use ui::UiState;

use horizon_client_render::hud::{HudAtlas, HudBatch, rgba};
use horizon_core::geometry::Aabb;
use winit::event::TouchPhase;

/// Core Starling Mod Menu instance embedded in the game client.
pub struct ModMenu {
    pub config: ModConfig,
    pub registry: FeatureRegistry,
    pub ui_state: UiState,
    pub combat: CombatSystem,
}

impl Default for ModMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl ModMenu {
    /// Creates and initializes the mod menu, loading persistent settings from disk.
    pub fn new() -> Self {
        let config = ModConfig::load();
        let mut registry = FeatureRegistry::new();
        registry.load_from_config(&config);

        let ui_state = UiState::new(&config);
        let combat = CombatSystem::new();

        Self {
            config,
            registry,
            ui_state,
            combat,
        }
    }

    /// Whether any visual feature (ESP, Target HUD, Coords) is currently enabled.
    pub fn visuals_active(&self) -> bool {
        self.registry.is_enabled("esp_box")
            || self.registry.is_enabled("esp_names")
            || self.registry.is_enabled("esp_health")
            || self.registry.is_enabled("target_hud")
            || self.registry.is_enabled("tracers")
            || self.registry.is_enabled("coords_hud")
    }

    /// Whether the menu window is open.
    pub fn is_open(&self) -> bool {
        self.ui_state.open
    }

    /// Toggles menu window visibility.
    pub fn toggle_open(&mut self) {
        self.ui_state.open = !self.ui_state.open;
        self.ui_state.save_to_config(&mut self.config);
        self.config.save();
    }

    /// Dispatches an Android touchscreen event.
    ///
    /// Coordinates `point` are in physical window pixels and are converted
    /// into GUI pixels based on `scale`. Returns `true` if the touch was
    /// handled by the menu or floating button, blocking underlying game controls.
    pub fn touch_event(
        &mut self,
        phase: TouchPhase,
        point: [f32; 2],
        touch_id: u64,
        scale: f32,
    ) -> bool {
        let s = scale.max(1.0);
        let gui_pt = [point[0] / s, point[1] / s];

        ui::handle_touch_event(
            phase,
            gui_pt,
            touch_id,
            &mut self.ui_state,
            &mut self.registry,
            &mut self.config,
        )
    }

    /// Renders world visuals: ESP boxes, nametags, health bars, tracers,
    /// target info HUD, and coordinates HUD.
    pub fn render_visuals(
        &mut self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cam_pos: [f64; 3],
        cam_view: [f32; 16],
        cam_proj: [f32; 16],
        player_pos: [f64; 3],
        player_yaw: f32,
        player_pitch: f32,
        biome_name: &str,
        fps: f32,
        mobs: &[(usize, u32, &str, Option<&str>, [f64; 3], Aabb, f32, f32)],
    ) {
        let screen_w = batch.width;
        let screen_h = batch.height;

        // Visual watermark brand
        visuals::draw_watermark(batch, atlas, &self.registry);

        // Coordinates & Heading HUD
        visuals::draw_coords_hud(
            batch,
            atlas,
            &self.registry,
            player_pos,
            player_yaw,
            player_pitch,
            biome_name,
            fps,
        );

        let view_proj = glam::Mat4::from_cols_array(&cam_proj) * glam::Mat4::from_cols_array(&cam_view);

        let esp_box = self.registry.is_enabled("esp_box");
        let esp_names = self.registry.is_enabled("esp_names");
        let esp_health = self.registry.is_enabled("esp_health");
        let show_tracers = self.registry.is_enabled("tracers");

        if esp_box || esp_names || esp_health || show_tracers {
            let box_style = self.registry.get_feature("esp_box").map_or(0, |f| f.get_enum_setting("box_style"));
            let thickness = self.registry.get_feature("esp_box").map_or(1.5, |f| f.get_float_setting("thickness"));

            let color_monster = self.registry.get_feature("esp_box").map_or([255, 60, 60, 230], |f| f.get_color_setting("color_monster"));
            let color_player = self.registry.get_feature("esp_box").map_or([60, 160, 255, 230], |f| f.get_color_setting("color_player"));
            let color_passive = self.registry.get_feature("esp_box").map_or([60, 240, 100, 230], |f| f.get_color_setting("color_passive"));

            let show_monsters = self.registry.get_feature("esp_box").map_or(true, |f| f.get_bool_setting("show_monsters"));
            let show_players = self.registry.get_feature("esp_box").map_or(true, |f| f.get_bool_setting("show_players"));
            let show_passives = self.registry.get_feature("esp_box").map_or(false, |f| f.get_bool_setting("show_passives"));

            let max_name_dist = self.registry.get_feature("esp_names").map_or(64.0, |f| f.get_float_setting("max_dist"));

            for &(_idx, _id, kind_id, custom_name, pos, bounds, health, max_health) in mobs {
                if health <= 0.0 {
                    continue;
                }

                let is_player = kind_id.ends_with("player");
                let is_passive = combat_is_passive(kind_id);
                let is_hostile = !is_player && !is_passive;

                if is_player && !show_players {
                    continue;
                }
                if is_hostile && !show_monsters {
                    continue;
                }
                if is_passive && !show_passives {
                    continue;
                }

                let color = if is_player {
                    rgba(color_player[0], color_player[1], color_player[2], color_player[3])
                } else if is_hostile {
                    rgba(color_monster[0], color_monster[1], color_monster[2], color_monster[3])
                } else {
                    rgba(color_passive[0], color_passive[1], color_passive[2], color_passive[3])
                };

                let dx = (pos[0] - cam_pos[0]) as f32;
                let dy = (pos[1] - cam_pos[1]) as f32;
                let dz = (pos[2] - cam_pos[2]) as f32;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();

                // 2D Bounding Box ESP
                if let Some(([x0, y0], [x1, y1])) = visuals::project_aabb(bounds, cam_pos, &view_proj, screen_w, screen_h) {
                    let bw = (x1 - x0).max(4.0);
                    let bh = (y1 - y0).max(4.0);

                    if esp_box {
                        if box_style == 1 {
                            visuals::draw_corner_box(batch, atlas, x0, y0, bw, bh, thickness, color);
                        } else {
                            visuals::draw_rect_outline(batch, atlas, x0, y0, bw, bh, thickness, color);
                        }
                    }

                    // Health Indicator
                    if esp_health {
                        visuals::draw_health_bar(batch, atlas, x0, y0, bh, health, max_health);
                    }

                    // Name and Distance Label
                    if esp_names && dist <= max_name_dist {
                        let entity_name = custom_name.unwrap_or_else(|| kind_id.trim_start_matches("minecraft:"));
                        let label_str = format!("{entity_name} [{dist:.0}m]");
                        let tw = HudBatch::text_width(atlas, &label_str);
                        let lx = x0 + (bw - tw) * 0.5;
                        let ly = y0 - 12.0;

                        batch.fill(atlas, lx - 2.0, ly - 1.0, tw + 4.0, 10.0, rgba(0, 0, 0, 150));
                        batch.text(atlas, &label_str, lx, ly, rgba(255, 255, 255, 255), true);
                    }

                    // Tracers
                    if show_tracers {
                        let _bottom_x = screen_w * 0.5;
                        let bottom_y = screen_h;
                        batch.fill(atlas, x0 + bw * 0.5, y1, 1.5, (bottom_y - y1).max(1.0), rgba(color_monster[0], color_monster[1], color_monster[2], 120));
                    }
                }
            }
        }

        // Target Info HUD
        if let Some(target) = &self.combat.current_target {
            visuals::draw_target_hud(
                batch,
                atlas,
                &self.registry,
                &target.name,
                &target.kind,
                target.health,
                target.max_health,
                target.distance,
            );
        }
    }

    /// Renders the floating open button and the main mod menu interface.
    pub fn render_menu(&mut self, batch: &mut HudBatch, atlas: &HudAtlas) {
        ui::draw_floating_button(batch, atlas, &self.ui_state);
        ui::draw_menu_window(batch, atlas, &mut self.ui_state, &self.registry);
    }
}

fn combat_is_passive(kind: &str) -> bool {
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
