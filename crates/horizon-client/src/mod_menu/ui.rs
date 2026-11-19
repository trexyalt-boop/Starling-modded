//! Modern dark-themed user interface and touch interaction system.

use super::config::ModConfig;
use super::feature::{FeatureCategory, FeatureRegistry, SettingValue};
use horizon_client_render::hud::{HudAtlas, HudBatch, rgba};
use winit::event::TouchPhase;

const fn c_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | (r as u32)
}

// Dark theme color palette
const COLOR_BG_WINDOW: u32 = c_rgba(15, 17, 24, 245);
const COLOR_BG_HEADER: u32 = c_rgba(22, 25, 36, 255);
const COLOR_BG_SIDEBAR: u32 = c_rgba(18, 20, 29, 255);
const COLOR_BG_CARD: u32 = c_rgba(25, 28, 40, 235);
const COLOR_BG_CARD_EXPANDED: u32 = c_rgba(20, 22, 32, 245);

const COLOR_ACCENT: u32 = c_rgba(64, 128, 255, 255);
const COLOR_ACCENT_HOVER: u32 = c_rgba(90, 150, 255, 255);
const COLOR_TOGGLE_ON: u32 = c_rgba(46, 204, 113, 255);
const COLOR_TOGGLE_OFF: u32 = c_rgba(55, 60, 75, 255);

const COLOR_TEXT_PRIMARY: u32 = c_rgba(245, 248, 255, 255);
const COLOR_TEXT_SECONDARY: u32 = c_rgba(155, 165, 185, 255);
const COLOR_BORDER: u32 = c_rgba(40, 45, 62, 200);

/// UI state managing window positioning, dragging, category selection, and touch events.
pub struct UiState {
    pub open: bool,
    pub collapsed: bool,
    pub window_x: f32,
    pub window_y: f32,
    pub window_w: f32,
    pub window_h: f32,
    pub floating_btn_x: f32,
    pub floating_btn_y: f32,
    pub active_category: FeatureCategory,
    pub search_query: String,
    pub scroll_offset: f32,

    // Touch interaction states
    pub dragging_window: bool,
    pub drag_start_touch: [f32; 2],
    pub drag_start_window: [f32; 2],
    pub dragging_button: bool,
    pub drag_start_button: [f32; 2],
    pub active_touch_id: Option<u64>,
    pub active_slider: Option<(String, String)>, // (feature_id, setting_key)
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            open: true,
            collapsed: false,
            window_x: 20.0,
            window_y: 20.0,
            window_w: 360.0,
            window_h: 210.0,
            floating_btn_x: 8.0,
            floating_btn_y: 18.0,
            active_category: FeatureCategory::Combat,
            search_query: String::new(),
            scroll_offset: 0.0,
            dragging_window: false,
            drag_start_touch: [0.0, 0.0],
            drag_start_window: [0.0, 0.0],
            dragging_button: false,
            drag_start_button: [0.0, 0.0],
            active_touch_id: None,
            active_slider: None,
        }
    }
}

impl UiState {
    pub fn new(config: &ModConfig) -> Self {
        let cat = match config.active_category {
            0 => FeatureCategory::Combat,
            1 => FeatureCategory::Visuals,
            2 => FeatureCategory::Player,
            3 => FeatureCategory::World,
            4 => FeatureCategory::Misc,
            _ => FeatureCategory::Settings,
        };

        Self {
            open: config.menu_open,
            collapsed: false,
            window_x: config.window_x.max(5.0),
            window_y: config.window_y.max(5.0),
            window_w: config.window_w.max(280.0),
            window_h: config.window_h.max(160.0),
            floating_btn_x: config.button_x.max(5.0),
            floating_btn_y: config.button_y.max(5.0),
            active_category: cat,
            search_query: String::new(),
            scroll_offset: 0.0,
            dragging_window: false,
            drag_start_touch: [0.0, 0.0],
            drag_start_window: [0.0, 0.0],
            dragging_button: false,
            drag_start_button: [0.0, 0.0],
            active_touch_id: None,
            active_slider: None,
        }
    }

    pub fn save_to_config(&self, config: &mut ModConfig) {
        config.menu_open = self.open;
        config.window_x = self.window_x;
        config.window_y = self.window_y;
        config.window_w = self.window_w;
        config.window_h = self.window_h;
        config.button_x = self.floating_btn_x;
        config.button_y = self.floating_btn_y;
        config.active_category = match self.active_category {
            FeatureCategory::Combat => 0,
            FeatureCategory::Visuals => 1,
            FeatureCategory::Player => 2,
            FeatureCategory::World => 3,
            FeatureCategory::Misc => 4,
            FeatureCategory::Settings => 5,
        };
    }
}

/// Renders the floating open/close toggle button.
pub fn draw_floating_button(batch: &mut HudBatch, atlas: &HudAtlas, state: &UiState) {
    let bx = state.floating_btn_x;
    let by = state.floating_btn_y;
    let bw = 64.0;
    let bh = 20.0;

    // Glowing border outline
    batch.fill(
        atlas,
        bx - 1.0,
        by - 1.0,
        bw + 2.0,
        bh + 2.0,
        rgba(64, 128, 255, 180),
    );
    // Dark pill surface
    let surface_color = if state.open {
        rgba(30, 45, 75, 240)
    } else {
        rgba(18, 22, 32, 230)
    };
    batch.fill(atlas, bx, by, bw, bh, surface_color);

    // Accent left strip
    batch.fill(atlas, bx, by, 3.0, bh, COLOR_ACCENT);

    // Label
    let label = if state.open { "✕ CLOSE" } else { "★ MODS" };
    let tw = HudBatch::text_width(atlas, label);
    batch.text(
        atlas,
        label,
        bx + (bw - tw) * 0.5 + 1.0,
        by + 6.0,
        COLOR_TEXT_PRIMARY,
        true,
    );
}

/// Renders the main draggable mod menu window.
pub fn draw_menu_window(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    state: &mut UiState,
    registry: &FeatureRegistry,
) {
    if !state.open {
        return;
    }

    let wx = state.window_x;
    let wy = state.window_y;
    let ww = state.window_w;
    let wh = if state.collapsed {
        24.0
    } else {
        state.window_h
    };

    // --- Window Outer Shadow & Background ---
    batch.fill(atlas, wx - 1.0, wy - 1.0, ww + 2.0, wh + 2.0, COLOR_BORDER);
    batch.fill(atlas, wx, wy, ww, wh, COLOR_BG_WINDOW);

    // --- Title Bar ---
    let title_h = 24.0;
    batch.fill(atlas, wx, wy, ww, title_h, COLOR_BG_HEADER);
    batch.fill(atlas, wx, wy, ww, 2.0, COLOR_ACCENT); // Top accent stripe

    // Window Title Text
    let title = "✦ STARLING NATIVE MOD MENU";
    batch.text(atlas, title, wx + 8.0, wy + 7.0, COLOR_TEXT_PRIMARY, true);

    // Minimize & Close Buttons
    let close_btn_w = 18.0;
    let close_x = wx + ww - close_btn_w - 4.0;
    let min_x = close_x - close_btn_w - 4.0;

    // Minimize [-]
    batch.fill(
        atlas,
        min_x,
        wy + 3.0,
        close_btn_w,
        18.0,
        rgba(40, 45, 60, 200),
    );
    batch.text(
        atlas,
        "—",
        min_x + 5.0,
        wy + 6.0,
        COLOR_TEXT_SECONDARY,
        false,
    );

    // Close [✕]
    batch.fill(
        atlas,
        close_x,
        wy + 3.0,
        close_btn_w,
        18.0,
        rgba(180, 50, 50, 220),
    );
    batch.text(
        atlas,
        "✕",
        close_x + 5.0,
        wy + 6.0,
        COLOR_TEXT_PRIMARY,
        false,
    );

    if state.collapsed {
        return;
    }

    // --- Sidebar (Categories) ---
    let sidebar_w = 72.0;
    let content_y = wy + title_h;
    let content_h = wh - title_h;
    batch.fill(atlas, wx, content_y, sidebar_w, content_h, COLOR_BG_SIDEBAR);
    batch.fill(
        atlas,
        wx + sidebar_w - 1.0,
        content_y,
        1.0,
        content_h,
        COLOR_BORDER,
    );

    let tab_h = 24.0;
    for (i, cat) in FeatureCategory::ALL.iter().enumerate() {
        let ty = content_y + 4.0 + (i as f32 * (tab_h + 3.0));
        let is_active = *cat == state.active_category;

        let tab_bg = if is_active {
            rgba(35, 45, 70, 255)
        } else {
            rgba(22, 25, 35, 180)
        };

        batch.fill(atlas, wx + 3.0, ty, sidebar_w - 6.0, tab_h, tab_bg);
        if is_active {
            batch.fill(atlas, wx + 3.0, ty, 2.5, tab_h, COLOR_ACCENT);
        }

        let cat_label = cat.display_name();
        let cat_color = if is_active {
            COLOR_TEXT_PRIMARY
        } else {
            COLOR_TEXT_SECONDARY
        };
        batch.text(atlas, cat_label, wx + 8.0, ty + 7.0, cat_color, is_active);
    }

    // --- Content Panel ---
    let panel_x = wx + sidebar_w + 4.0;
    let panel_w = ww - sidebar_w - 8.0;
    let panel_y = content_y + 4.0;

    // Search Bar / Active Header
    let search_h = 16.0;
    batch.fill(
        atlas,
        panel_x,
        panel_y,
        panel_w,
        search_h,
        rgba(20, 22, 32, 220),
    );
    batch.fill(atlas, panel_x, panel_y, 1.0, search_h, COLOR_BORDER);

    let search_display = if state.search_query.is_empty() {
        format!(
            "Category: {} (Tap to toggle)",
            state.active_category.display_name()
        )
    } else {
        format!("Search: {}", state.search_query)
    };
    batch.text(
        atlas,
        &search_display,
        panel_x + 6.0,
        panel_y + 4.0,
        COLOR_TEXT_SECONDARY,
        false,
    );

    // List of features in active category
    let list_y = panel_y + search_h + 4.0;
    let list_h = content_h - search_h - 12.0;

    let mut current_y = list_y - state.scroll_offset;

    for feature in &registry.features {
        // Filter by category or search query
        if !state.search_query.is_empty() {
            let q = state.search_query.to_lowercase();
            if !feature.name.to_lowercase().contains(&q) && !feature.id.to_lowercase().contains(&q)
            {
                continue;
            }
        } else if feature.category != state.active_category {
            continue;
        }

        let card_h = if feature.expanded && !feature.settings.is_empty() {
            28.0 + (feature.settings.len() as f32 * 20.0)
        } else {
            26.0
        };

        // Clip cards outside viewable area
        if current_y + card_h >= list_y && current_y <= list_y + list_h {
            let card_bg = if feature.expanded {
                COLOR_BG_CARD_EXPANDED
            } else {
                COLOR_BG_CARD
            };

            batch.fill(atlas, panel_x, current_y, panel_w, card_h, card_bg);
            batch.fill(
                atlas,
                panel_x,
                current_y,
                2.0,
                card_h,
                if feature.enabled {
                    COLOR_TOGGLE_ON
                } else {
                    COLOR_BORDER
                },
            );

            // Feature Name
            let name_color = if feature.enabled {
                COLOR_TEXT_PRIMARY
            } else {
                COLOR_TEXT_SECONDARY
            };
            batch.text(
                atlas,
                &feature.name,
                panel_x + 8.0,
                current_y + 4.0,
                name_color,
                feature.enabled,
            );

            // Toggle switch on the right
            let switch_w = 34.0;
            let switch_h = 16.0;
            let switch_x = panel_x + panel_w - switch_w - 6.0;
            let switch_y = current_y + 4.0;

            let switch_bg = if feature.enabled {
                COLOR_TOGGLE_ON
            } else {
                COLOR_TOGGLE_OFF
            };
            batch.fill(atlas, switch_x, switch_y, switch_w, switch_h, switch_bg);

            // Toggle knob
            let knob_x = if feature.enabled {
                switch_x + switch_w - 14.0
            } else {
                switch_x + 2.0
            };
            batch.fill(
                atlas,
                knob_x,
                switch_y + 2.0,
                12.0,
                switch_h - 4.0,
                rgba(255, 255, 255, 255),
            );

            let status_text = if feature.enabled { "ON" } else { "OFF" };
            let st_color = if feature.enabled {
                rgba(20, 30, 20, 255)
            } else {
                rgba(180, 185, 195, 255)
            };
            batch.text(
                atlas,
                status_text,
                switch_x + 6.0,
                switch_y + 4.0,
                st_color,
                false,
            );

            // Options Expander Button if feature has settings
            if !feature.settings.is_empty() {
                let opt_btn_w = 16.0;
                let opt_x = switch_x - opt_btn_w - 4.0;
                batch.fill(
                    atlas,
                    opt_x,
                    switch_y,
                    opt_btn_w,
                    switch_h,
                    rgba(35, 40, 55, 200),
                );
                let opt_char = if feature.expanded { "▲" } else { "▼" };
                batch.text(
                    atlas,
                    opt_char,
                    opt_x + 4.0,
                    switch_y + 4.0,
                    COLOR_TEXT_SECONDARY,
                    false,
                );
            }

            // Expanded settings rows
            if feature.expanded {
                let mut set_y = current_y + 26.0;
                for setting in &feature.settings {
                    batch.fill(
                        atlas,
                        panel_x + 8.0,
                        set_y,
                        panel_w - 16.0,
                        1.0,
                        rgba(40, 45, 60, 150),
                    );

                    match &setting.value {
                        SettingValue::Float {
                            value, min, max, ..
                        } => {
                            let label = format!("{}: {:.1}", setting.name, value);
                            batch.text(
                                atlas,
                                &label,
                                panel_x + 10.0,
                                set_y + 4.0,
                                COLOR_TEXT_SECONDARY,
                                false,
                            );

                            // Slider track
                            let track_w = 80.0;
                            let track_x = panel_x + panel_w - track_w - 10.0;
                            let track_y = set_y + 6.0;
                            let track_h = 6.0;

                            batch.fill(
                                atlas,
                                track_x,
                                track_y,
                                track_w,
                                track_h,
                                rgba(30, 34, 46, 255),
                            );
                            let progress = ((*value - min) / (max - min).max(0.01)).clamp(0.0, 1.0);
                            batch.fill(
                                atlas,
                                track_x,
                                track_y,
                                track_w * progress,
                                track_h,
                                COLOR_ACCENT,
                            );

                            // Slider knob
                            batch.fill(
                                atlas,
                                track_x + track_w * progress - 3.0,
                                track_y - 2.0,
                                6.0,
                                10.0,
                                rgba(255, 255, 255, 255),
                            );
                        }
                        SettingValue::Enum { selected, options } => {
                            batch.text(
                                atlas,
                                &setting.name,
                                panel_x + 10.0,
                                set_y + 4.0,
                                COLOR_TEXT_SECONDARY,
                                false,
                            );

                            let opt_str = options
                                .get(*selected)
                                .map(|s| s.as_str())
                                .unwrap_or("Default");
                            let opt_w = HudBatch::text_width(atlas, opt_str) + 12.0;
                            let opt_x = panel_x + panel_w - opt_w - 10.0;
                            batch.fill(
                                atlas,
                                opt_x,
                                set_y + 3.0,
                                opt_w,
                                13.0,
                                rgba(38, 44, 60, 240),
                            );
                            batch.text(
                                atlas,
                                opt_str,
                                opt_x + 6.0,
                                set_y + 4.0,
                                COLOR_ACCENT_HOVER,
                                false,
                            );
                        }
                        SettingValue::Bool(b) => {
                            batch.text(
                                atlas,
                                &setting.name,
                                panel_x + 10.0,
                                set_y + 4.0,
                                COLOR_TEXT_SECONDARY,
                                false,
                            );

                            let check_x = panel_x + panel_w - 24.0;
                            let check_bg = if *b {
                                COLOR_TOGGLE_ON
                            } else {
                                COLOR_TOGGLE_OFF
                            };
                            batch.fill(atlas, check_x, set_y + 3.0, 14.0, 14.0, check_bg);
                            if *b {
                                batch.text(
                                    atlas,
                                    "✓",
                                    check_x + 3.0,
                                    set_y + 4.0,
                                    rgba(255, 255, 255, 255),
                                    true,
                                );
                            }
                        }
                        SettingValue::Color(c) => {
                            batch.text(
                                atlas,
                                &setting.name,
                                panel_x + 10.0,
                                set_y + 4.0,
                                COLOR_TEXT_SECONDARY,
                                false,
                            );
                            let swatch_x = panel_x + panel_w - 24.0;
                            batch.fill(
                                atlas,
                                swatch_x,
                                set_y + 3.0,
                                14.0,
                                14.0,
                                rgba(c[0], c[1], c[2], c[3]),
                            );
                        }
                    }
                    set_y += 20.0;
                }
            }
        }
        current_y += card_h + 3.0;
    }
}

/// Dispatches touchscreen events to the mod menu. Returns `true` if the event
/// was intercepted and consumed by the menu or floating button.
pub fn handle_touch_event(
    phase: TouchPhase,
    point: [f32; 2],
    touch_id: u64,
    state: &mut UiState,
    registry: &mut FeatureRegistry,
    config: &mut ModConfig,
) -> bool {
    let [px, py] = point;

    // --- 1. Check Floating Toggle Button ---
    let btn_x = state.floating_btn_x;
    let btn_y = state.floating_btn_y;
    let btn_w = 64.0;
    let btn_h = 20.0;

    let in_button = px >= btn_x && px <= btn_x + btn_w && py >= btn_y && py <= btn_y + btn_h;

    match phase {
        TouchPhase::Started => {
            if in_button {
                state.active_touch_id = Some(touch_id);
                state.dragging_button = true;
                state.drag_start_touch = [px, py];
                state.drag_start_button = [btn_x, btn_y];
                return true;
            }
        }
        TouchPhase::Moved => {
            if state.dragging_button && state.active_touch_id == Some(touch_id) {
                let dx = px - state.drag_start_touch[0];
                let dy = py - state.drag_start_touch[1];
                state.floating_btn_x = (state.drag_start_button[0] + dx).max(2.0);
                state.floating_btn_y = (state.drag_start_button[1] + dy).max(2.0);
                return true;
            }
        }
        TouchPhase::Ended | TouchPhase::Cancelled => {
            if state.dragging_button && state.active_touch_id == Some(touch_id) {
                state.dragging_button = false;
                state.active_touch_id = None;

                let dx = (px - state.drag_start_touch[0]).abs();
                let dy = (py - state.drag_start_touch[1]).abs();
                // If finger didn't drag significantly, treat as tap toggle
                if dx < 6.0 && dy < 6.0 {
                    state.open = !state.open;
                    state.save_to_config(config);
                    config.save();
                }
                return true;
            }
        }
    }

    if !state.open {
        return false;
    }

    // --- 2. Check Mod Menu Window ---
    let wx = state.window_x;
    let wy = state.window_y;
    let ww = state.window_w;
    let wh = if state.collapsed {
        24.0
    } else {
        state.window_h
    };

    let in_window = px >= wx && px <= wx + ww && py >= wy && py <= wy + wh;

    match phase {
        TouchPhase::Started => {
            if in_window {
                state.active_touch_id = Some(touch_id);

                // Check title bar (drag window or window buttons)
                if py <= wy + 24.0 {
                    let close_x = wx + ww - 22.0;
                    let min_x = close_x - 22.0;

                    if px >= close_x {
                        // Close [✕]
                        state.open = false;
                        state.save_to_config(config);
                        config.save();
                        return true;
                    }
                    if px >= min_x && px < close_x {
                        // Minimize [—]
                        state.collapsed = !state.collapsed;
                        return true;
                    }

                    // Otherwise start dragging window
                    state.dragging_window = true;
                    state.drag_start_touch = [px, py];
                    state.drag_start_window = [wx, wy];
                    return true;
                }

                if state.collapsed {
                    return true;
                }

                // Check Category Sidebar
                let sidebar_w = 72.0;
                if px <= wx + sidebar_w {
                    let tab_h = 24.0;
                    for (i, cat) in FeatureCategory::ALL.iter().enumerate() {
                        let ty = wy + 24.0 + 4.0 + (i as f32 * (tab_h + 3.0));
                        if py >= ty && py <= ty + tab_h {
                            state.active_category = *cat;
                            state.scroll_offset = 0.0;
                            state.save_to_config(config);
                            config.save();
                            return true;
                        }
                    }
                    return true;
                }

                // Check Content Panel Interactions (Feature toggles, expanders, sliders)
                let panel_x = wx + sidebar_w + 4.0;
                let panel_w = ww - sidebar_w - 8.0;
                let list_y = wy + 24.0 + 4.0 + 20.0;

                let mut current_y = list_y - state.scroll_offset;

                for feature in &mut registry.features {
                    if !state.search_query.is_empty() {
                        let q = state.search_query.to_lowercase();
                        if !feature.name.to_lowercase().contains(&q)
                            && !feature.id.to_lowercase().contains(&q)
                        {
                            continue;
                        }
                    } else if feature.category != state.active_category {
                        continue;
                    }

                    let card_h = if feature.expanded && !feature.settings.is_empty() {
                        28.0 + (feature.settings.len() as f32 * 20.0)
                    } else {
                        26.0
                    };

                    if py >= current_y && py <= current_y + card_h {
                        let switch_w = 34.0;
                        let switch_x = panel_x + panel_w - switch_w - 6.0;

                        // Tap toggle switch
                        if px >= switch_x && px <= switch_x + switch_w && py <= current_y + 24.0 {
                            feature.enabled = !feature.enabled;
                            registry.save_to_config(config);
                            config.save();
                            return true;
                        }

                        // Tap options expander
                        if !feature.settings.is_empty() {
                            let opt_x = switch_x - 20.0;
                            if px >= opt_x && px < switch_x && py <= current_y + 24.0 {
                                feature.expanded = !feature.expanded;
                                return true;
                            }
                        }

                        // Check expanded settings taps
                        if feature.expanded {
                            let mut set_y = current_y + 26.0;
                            for setting in &mut feature.settings {
                                if py >= set_y && py <= set_y + 20.0 {
                                    match &mut setting.value {
                                        SettingValue::Bool(b) => {
                                            *b = !*b;
                                            registry.save_to_config(config);
                                            config.save();
                                            return true;
                                        }
                                        SettingValue::Enum { selected, options } => {
                                            if !options.is_empty() {
                                                *selected = (*selected + 1) % options.len();
                                                registry.save_to_config(config);
                                                config.save();
                                                return true;
                                            }
                                        }
                                        SettingValue::Float {
                                            value, min, max, ..
                                        } => {
                                            let track_w = 80.0;
                                            let track_x = panel_x + panel_w - track_w - 10.0;
                                            if px >= track_x - 5.0 && px <= track_x + track_w + 5.0
                                            {
                                                state.active_slider =
                                                    Some((feature.id.clone(), setting.key.clone()));
                                                let pct =
                                                    ((px - track_x) / track_w).clamp(0.0, 1.0);
                                                *value = *min + pct * (*max - *min);
                                                registry.save_to_config(config);
                                                config.save();
                                                return true;
                                            }
                                        }
                                        SettingValue::Color(_) => {}
                                    }
                                }
                                set_y += 20.0;
                            }
                        }
                    }
                    current_y += card_h + 3.0;
                }

                return true;
            }
        }
        TouchPhase::Moved => {
            if state.active_touch_id == Some(touch_id) {
                if state.dragging_window {
                    let dx = px - state.drag_start_touch[0];
                    let dy = py - state.drag_start_touch[1];
                    state.window_x = (state.drag_start_window[0] + dx).max(0.0);
                    state.window_y = (state.drag_start_window[1] + dy).max(0.0);
                    return true;
                }

                // Dragging a slider
                if let Some((feat_id, set_key)) = &state.active_slider {
                    if let Some(f) = registry.get_feature_mut(feat_id) {
                        for s in &mut f.settings {
                            if s.key == *set_key {
                                if let SettingValue::Float {
                                    value, min, max, ..
                                } = &mut s.value
                                {
                                    let sidebar_w = 72.0;
                                    let panel_x = wx + sidebar_w + 4.0;
                                    let panel_w = ww - sidebar_w - 8.0;
                                    let track_w = 80.0;
                                    let track_x = panel_x + panel_w - track_w - 10.0;

                                    let pct = ((px - track_x) / track_w).clamp(0.0, 1.0);
                                    *value = *min + pct * (*max - *min);
                                    registry.save_to_config(config);
                                    config.save();
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            if in_window {
                return true;
            }
        }
        TouchPhase::Ended | TouchPhase::Cancelled => {
            if state.active_touch_id == Some(touch_id) {
                state.dragging_window = false;
                state.active_slider = None;
                state.active_touch_id = None;
                state.save_to_config(config);
                config.save();
                return true;
            }
            if in_window {
                return true;
            }
        }
    }

    in_window
}
