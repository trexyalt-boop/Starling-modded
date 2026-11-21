//! CorzClient-style ClickGUI: a dark glass 620x360 panel with a category
//! sidebar, module cards, search, typed setting rows, dropdowns, a colour
//! picker, keybind capture, toasts and touchscreen dragging/scrolling.
//!
//! The layout is responsive: on a small phone the panel shrinks to the
//! screen, keeping the same proportions and touch targets.

use super::config::ModConfig;
use super::feature::{Feature, FeatureCategory, FeatureRegistry, SettingValue};
use super::notifications::Notifications;
use super::theme::{self, Palette};
use horizon_client_render::hud::{HudAtlas, HudBatch, rgba};
use winit::event::TouchPhase;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundKind {
    Hover,
    Click,
    ToggleOn,
    ToggleOff,
}

#[derive(Clone, Debug)]
pub enum UiAction {
    SaveConfig,
    ReloadConfig,
    ResetConfig,
    Panic,
    AddFriend,
    RemoveFriend(String),
    AddWaypoint,
    SetAccent([u8; 4]),
}

#[derive(Clone, Debug)]
pub enum BindTarget {
    Module(String),
    Setting(String, String),
}

#[derive(Clone, Debug)]
pub struct Dropdown {
    pub feature: String,
    pub key: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub options: Vec<String>,
    pub selected: usize,
}

#[derive(Clone, Debug)]
pub struct ColorPicker {
    pub feature: String,
    pub key: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [u8; 4],
}

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
    pub search_focus: bool,
    pub scroll_offset: f32,

    // Touch interaction state.
    pub dragging_window: bool,
    pub drag_start_touch: [f32; 2],
    pub drag_start_window: [f32; 2],
    pub dragging_button: bool,
    pub drag_start_button: [f32; 2],
    pub active_touch_id: Option<u64>,
    pub active_slider: Option<(String, String)>,
    pub dragging_scroll: bool,
    pub scroll_start_touch: f32,
    pub scroll_start_offset: f32,
    pub scroll_moved: bool,
    pub resizing: bool,
    pub resize_start_touch: [f32; 2],
    pub resize_start_size: [f32; 2],

    // Overlays and capture.
    pub dropdown: Option<Dropdown>,
    pub color_picker: Option<ColorPicker>,
    pub listening: Option<BindTarget>,

    // Animation / feedback.
    pub open_t: f32,
    pub time: f32,
    pub last_tick: std::time::Instant,
    pub hover: Option<String>,
    pub screen_w: f32,
    pub screen_h: f32,
    pub content_h: f32,

    pub pending_toast: Option<(String, String)>,
    pub pending_sound: Option<SoundKind>,
    pub pending_action: Option<UiAction>,

    // Cached copies of the config, so drawing needs no `&ModConfig`.
    pub cached_friends: Vec<String>,
    pub cached_accent: [u8; 4],
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            open: true,
            collapsed: false,
            window_x: 20.0,
            window_y: 16.0,
            window_w: 620.0,
            window_h: 360.0,
            floating_btn_x: 10.0,
            floating_btn_y: 10.0,
            active_category: FeatureCategory::Combat,
            search_query: String::new(),
            search_focus: false,
            scroll_offset: 0.0,
            dragging_window: false,
            drag_start_touch: [0.0, 0.0],
            drag_start_window: [0.0, 0.0],
            dragging_button: false,
            drag_start_button: [0.0, 0.0],
            active_touch_id: None,
            active_slider: None,
            dragging_scroll: false,
            scroll_start_touch: 0.0,
            scroll_start_offset: 0.0,
            scroll_moved: false,
            resizing: false,
            resize_start_touch: [0.0, 0.0],
            resize_start_size: [0.0, 0.0],
            dropdown: None,
            color_picker: None,
            listening: None,
            open_t: 0.0,
            time: 0.0,
            last_tick: std::time::Instant::now(),
            hover: None,
            screen_w: 640.0,
            screen_h: 400.0,
            content_h: 0.0,
            pending_toast: None,
            pending_sound: None,
            pending_action: None,
            cached_friends: Vec::new(),
            cached_accent: [124, 92, 255, 255],
        }
    }
}

impl UiState {
    pub fn new(config: &ModConfig) -> Self {
        let mut s = Self::default();
        s.open = config.menu_open;
        s.open_t = if s.open { 1.0 } else { 0.0 };
        s.window_x = config.window_x.max(2.0);
        s.window_y = config.window_y.max(2.0);
        s.window_w = config.window_w.max(280.0);
        s.window_h = config.window_h.max(180.0);
        s.floating_btn_x = config.button_x.max(2.0);
        s.floating_btn_y = config.button_y.max(2.0);
        s.active_category = FeatureCategory::from_index(config.active_category);
        s.cached_friends = config.friends.clone();
        s.cached_accent = config.accent;
        s
    }

    pub fn save_to_config(&self, config: &mut ModConfig) {
        config.menu_open = self.open;
        config.window_x = self.window_x;
        config.window_y = self.window_y;
        config.window_w = self.window_w;
        config.window_h = self.window_h;
        config.button_x = self.floating_btn_x;
        config.button_y = self.floating_btn_y;
        config.active_category = self.active_category.index();
    }

    pub fn tick(&mut self, dt: f32, animations: bool) {
        let dt = dt.clamp(0.0, 0.2);
        self.time += dt;
        let target = if self.open { 1.0 } else { 0.0 };
        if animations {
            let rate = 12.0;
            self.open_t += (target - self.open_t) * (1.0 - (-rate * dt).exp());
            if (target - self.open_t).abs() < 0.002 {
                self.open_t = target;
            }
        } else {
            self.open_t = target;
        }
        self.last_tick = std::time::Instant::now();
    }

    pub fn take_sound(&mut self) -> Option<SoundKind> {
        self.pending_sound.take()
    }

    pub fn take_toast(&mut self) -> Option<(String, String)> {
        self.pending_toast.take()
    }

    pub fn take_action(&mut self) -> Option<UiAction> {
        self.pending_action.take()
    }

    fn toast(&mut self, title: &str, msg: &str) {
        self.pending_toast = Some((title.to_string(), msg.to_string()));
    }

    fn sound(&mut self, kind: SoundKind) {
        self.pending_sound = Some(kind);
    }

    /// Handles a typed character while the search field has focus.
    pub fn text_input(&mut self, ch: char) -> bool {
        if !self.search_focus || !self.open {
            return false;
        }
        if ch.is_ascii_graphic() || ch == ' ' {
            if self.search_query.chars().count() < 28 {
                self.search_query.push(ch);
            }
        }
        true
    }

    pub fn backspace(&mut self) -> bool {
        if !self.search_focus || !self.open {
            return false;
        }
        self.search_query.pop();
        true
    }

    /// Handles a captured key. Returns true when the menu consumed it.
    pub fn key_event(&mut self, name: &str, registry: &mut FeatureRegistry) -> bool {
        if !self.open {
            return false;
        }
        if name == "Escape" {
            if self.listening.take().is_some() {
                return true;
            }
            if self.search_focus {
                self.search_focus = false;
                return true;
            }
            if self.dropdown.take().is_some() || self.color_picker.take().is_some() {
                return true;
            }
            return false;
        }
        let Some(target) = self.listening.take() else {
            return false;
        };
        let label = pretty_key(name);
        match target {
            BindTarget::Module(id) => {
                if let Some(f) = registry.get_feature_mut(&id) {
                    f.bind = label.clone();
                    self.toast("Bind Set", &format!("{} → {}", f.name, label));
                }
            }
            BindTarget::Setting(fid, key) => {
                if let Some(f) = registry.get_feature_mut(&fid) {
                    if let Some(s) = f.setting_mut(&key) {
                        s.value = SettingValue::Keybind(label.clone());
                        self.toast("Bind Set", &format!("{} → {}", s.name, label));
                    }
                }
            }
        }
        self.sound(SoundKind::Click);
        true
    }

}

pub fn pretty_key(name: &str) -> String {
    if name.is_empty() || name == "None" {
        return "None".to_string();
    }
    let mouse = match name {
        "key.mouse.left" => Some("LMB"),
        "key.mouse.right" => Some("RMB"),
        "key.mouse.middle" => Some("MMB"),
        "key.mouse.4" => Some("M4"),
        "key.mouse.5" => Some("M5"),
        _ => None,
    };
    if let Some(m) = mouse {
        return m.to_string();
    }
    let raw = name.strip_prefix("key.keyboard.").unwrap_or(name);
    let raw = match raw.strip_prefix("Key") {
        Some(rest) if rest.chars().count() == 1 => rest,
        _ => raw,
    };
    match raw {
        "left.shift" => "LShift".to_string(),
        "right.shift" => "RShift".to_string(),
        "left.control" => "LCtrl".to_string(),
        "right.control" => "RCtrl".to_string(),
        "left.alt" => "LAlt".to_string(),
        "right.alt" => "RAlt".to_string(),
        "space" => "Space".to_string(),
        "tab" => "Tab".to_string(),
        "escape" => "Esc".to_string(),
        "enter" => "Enter".to_string(),
        "backspace" => "Bksp".to_string(),
        "caps.lock" => "Caps".to_string(),
        "arrow.up" => "Up".to_string(),
        "arrow.down" => "Down".to_string(),
        "arrow.left" => "Left".to_string(),
        "arrow.right" => "Right".to_string(),
        "numpad.enter" => "NEnter".to_string(),
        "page.up" => "PgUp".to_string(),
        "page.down" => "PgDn".to_string(),
        other => {
            if other.chars().count() == 1 {
                other.to_uppercase()
            } else {
                other.to_string()
            }
        }
    }
}

struct Layout {
    wx: f32,
    wy: f32,
    ww: f32,
    wh: f32,
    header: f32,
    sidebar: f32,
    list_x: f32,
    list_w: f32,
}

impl Layout {
    fn new(state: &mut UiState, screen_w: f32, screen_h: f32) -> Self {
        // Keep the configured size but never larger than the screen.
        let max_w = (screen_w - 6.0).max(280.0);
        let max_h = (screen_h - 6.0).max(180.0);
        let ww = state.window_w.min(max_w).max(280.0);
        let wh = state.window_h.min(max_h).max(180.0);
        state.window_w = ww;
        state.window_h = wh;
        state.window_x = state.window_x.clamp(0.0, (screen_w - ww).max(0.0));
        state.window_y = state.window_y.clamp(0.0, (screen_h - wh).max(0.0));
        let header = 30.0;
        let sidebar = (ww * 0.24).clamp(84.0, 150.0);
        Self {
            wx: state.window_x,
            wy: state.window_y,
            ww,
            wh,
            header,
            sidebar,
            list_x: state.window_x + sidebar + 8.0,
            list_w: ww - sidebar - 16.0,
        }
    }
}

fn card_height(f: &Feature) -> f32 {
    let mut h = 28.0;
    if f.expanded {
        h += 12.0;
        if f.id == "configs" {
            h += 24.0;
        } else if f.id == "friends" {
            h += 20.0 + 18.0 * 4.0;
        } else if f.id == "gui_settings" {
            h += 22.0;
        } else {
            h += f.settings.len() as f32 * 26.0;
        }
    }
    h
}

fn matches(f: &Feature, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    f.name.to_lowercase().contains(&q)
        || f.id.to_lowercase().contains(&q)
        || f.description.to_lowercase().contains(&q)
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn visible<'a>(
    registry: &'a FeatureRegistry,
    state: &UiState,
) -> Vec<&'a Feature> {
    visible_indices(registry, state)
        .into_iter()
        .map(|i| &registry.features[i])
        .collect()
}

/// Registry indices of the features shown, sorted by name. Used where the
/// registry must be borrowed mutably afterwards.
fn visible_indices(registry: &FeatureRegistry, state: &UiState) -> Vec<usize> {
    let mut idx: Vec<usize> = registry
        .features
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            if state.search_query.is_empty() {
                f.category == state.active_category
            } else {
                matches(f, &state.search_query)
            }
        })
        .map(|(i, _)| i)
        .collect();
    idx.sort_by(|&a, &b| registry.features[a].name.cmp(&registry.features[b].name));
    idx
}

fn content_height(items: &[&Feature]) -> f32 {
    items.iter().map(|f| card_height(f) + 4.0).sum()
}

/// Draws the floating open/close button.
pub fn draw_floating_button(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    state: &UiState,
    palette: &Palette,
) {
    let bx = state.floating_btn_x;
    let by = state.floating_btn_y;
    let bw = 62.0;
    let bh = 22.0;
    if state.open {
        return;
    }
    theme::glow(batch, atlas, bx, by, bw, bh, palette.accent, state.time, 0.7);
    theme::rounded_rect(batch, atlas, bx, by, bw, bh, 7.0, palette.header);
    batch.fill(atlas, bx, by + 4.0, 2.0, bh - 8.0, palette.accent);
    batch.text(atlas, "CORZ", bx + 10.0, by + 7.0, palette.text, true);
    // A small status dot.
    let on = 0.55 + 0.45 * (state.time * 2.0).sin();
    theme::rounded_rect(
        batch,
        atlas,
        bx + bw - 12.0,
        by + 8.0,
        6.0,
        6.0,
        3.0,
        theme::with_alpha(palette.accent, (on * 220.0) as u8),
    );
}

/// Draws the whole window. `animations` comes from the GUI settings.
#[allow(clippy::too_many_arguments)]
pub fn draw_menu(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    state: &mut UiState,
    registry: &FeatureRegistry,
    palette: &Palette,
    animations: bool,
) {
    if !state.open {
        return;
    }
    let screen_w = batch.width;
    let screen_h = batch.height;
    state.screen_w = screen_w;
    state.screen_h = screen_h;
    let l = Layout::new(state, screen_w, screen_h);

    let items = visible(registry, state);
    state.content_h = content_height(&items);

    let anim = if animations { state.open_t } else { 1.0 };
    let ease = 1.0 - (1.0 - anim).powi(3);
    let grow = 0.94 + 0.06 * ease;
    let cx = l.wx + l.ww * 0.5;
    let cy = l.wy + l.wh * 0.5;
    let wx = cx - l.ww * grow * 0.5;
    let wy = cy - l.wh * grow * 0.5;
    let ww = l.ww * grow;
    let wh = l.wh * grow;
    let a = |c: u32| theme::scale_alpha(c, ease);

    // Ambient glow and shadow behind the glass.
    theme::shadow(batch, atlas, wx, wy, ww, wh, 12.0, rgba(0, 0, 0, 170));
    theme::glow(batch, atlas, wx, wy, ww, wh, palette.accent, state.time, 0.8);
    theme::rounded_rect(batch, atlas, wx, wy, ww, wh, 12.0, a(palette.window));
    // A hairline border.
    for (dx, dy, w, h) in [
        (wx, wy, ww, 1.0),
        (wx, wy + wh - 1.0, ww, 1.0),
        (wx, wy, 1.0, wh),
        (wx + ww - 1.0, wy, 1.0, wh),
    ] {
        batch.fill(atlas, dx, dy, w, h, a(palette.border));
    }

    // Content first: the header and sidebar then cover any card that scrolls
    // under them, which is the scissor this renderer does not expose.
    let list_x = wx + l.sidebar + 8.0;
    let list_y = wy + l.header + 6.0;
    let list_w = ww - l.sidebar - 16.0;
    let list_h = wh - l.header - 12.0;

    let max_scroll = (state.content_h - list_h).max(0.0);
    state.scroll_offset = state.scroll_offset.clamp(0.0, max_scroll);

    let mut y = list_y - state.scroll_offset;
    for f in &items {
        let h = card_height(f);
        if y + h >= list_y && y <= list_y + list_h {
            draw_card(
                batch, atlas, state, f, list_x, y, list_w, h, palette, a,
            );
        }
        y += h + 4.0;
    }

    // Bottom mask so cards do not spill out of the window.
    if state.scroll_offset < max_scroll || max_scroll > 0.0 {
        batch.fill(
            atlas,
            list_x - 2.0,
            list_y + list_h,
            list_w + 4.0,
            wy + wh - (list_y + list_h),
            a(palette.window),
        );
    }

    // Header.
    theme::rounded_rect(batch, atlas, wx, wy, ww, l.header, 12.0, a(palette.header));
    batch.fill(atlas, wx, wy + l.header - 12.0, ww, 12.0, a(palette.header));
    batch.fill(atlas, wx, wy + l.header - 1.0, ww, 1.0, a(palette.border_soft));
    // Logo mark.
    theme::rounded_rect(batch, atlas, wx + 9.0, wy + 8.0, 14.0, 14.0, 4.0, a(palette.accent));
    theme::rounded_rect(batch, atlas, wx + 12.0, wy + 11.0, 8.0, 8.0, 2.0, a(palette.header));
    batch.text(atlas, "CORZ", wx + 28.0, wy + 10.0, a(palette.text), true);
    batch.text(atlas, "2.0", wx + 30.0 + HudBatch::text_width(atlas, "CORZ"), wy + 11.0, a(palette.text_muted), false);

    // Search pill.
    let search_w = (ww * 0.32).clamp(90.0, 190.0);
    let search_x = wx + ww - search_w - 56.0;
    let search_y = wy + 7.0;
    theme::rounded_rect(
        batch,
        atlas,
        search_x,
        search_y,
        search_w,
        16.0,
        8.0,
        a(if state.search_focus {
            palette.card_hover
        } else {
            palette.card
        }),
    );
    if state.search_focus {
        batch.fill(atlas, search_x, search_y + 3.0, 1.5, 10.0, a(palette.accent));
    }
    let query = if state.search_query.is_empty() {
        "Search modules…".to_string()
    } else {
        state.search_query.clone()
    };
    let qcol = if state.search_query.is_empty() {
        a(palette.text_muted)
    } else {
        a(palette.text)
    };
    batch.text(atlas, &truncate(&query, 24), search_x + 8.0, search_y + 4.0, qcol, false);

    // Minimise and close.
    let close_x = wx + ww - 24.0;
    let min_x = close_x - 22.0;
    theme::rounded_rect(batch, atlas, min_x, wy + 7.0, 18.0, 16.0, 5.0, a(palette.card));
    batch.fill(atlas, min_x + 5.0, wy + 14.0, 8.0, 1.5, a(palette.text_dim));
    theme::rounded_rect(batch, atlas, close_x, wy + 7.0, 18.0, 16.0, 5.0, a(rgba(180, 50, 60, 255)));
    let xx = close_x + 5.0;
    let xy = wy + 11.0;
    batch.fill_quad(atlas, [[xx, xy], [xx + 8.0, xy + 8.0], [xx + 7.0, xy + 9.0], [xx - 1.0, xy + 1.0]], a(palette.text));
    batch.fill_quad(atlas, [[xx + 8.0, xy], [xx, xy + 8.0], [xx - 1.0, xy + 7.0], [xx + 7.0, xy - 1.0]], a(palette.text));

    // Sidebar.
    let sidebar_x = wx;
    let sidebar_y = wy + l.header;
    let sidebar_h = wh - l.header;
    theme::rounded_rect(batch, atlas, sidebar_x, sidebar_y, l.sidebar, sidebar_h, 10.0, a(palette.sidebar));
    batch.fill(atlas, sidebar_x + l.sidebar - 1.0, sidebar_y, 1.0, sidebar_h, a(palette.border_soft));

    let mut cat_y = sidebar_y + 6.0;
    for cat in FeatureCategory::ALL {
        let active = cat == state.active_category && state.search_query.is_empty();
        let row_h = 26.0;
        if active {
            theme::rounded_rect(
                batch,
                atlas,
                sidebar_x + 5.0,
                cat_y,
                l.sidebar - 10.0,
                row_h,
                6.0,
                a(palette.accent_dim),
            );
            batch.fill(atlas, sidebar_x + 5.0, cat_y + 4.0, 2.5, row_h - 8.0, a(palette.accent));
        }
        let icon_color = if active { a(palette.accent) } else { a(palette.text_dim) };
        theme::category_icon(batch, atlas, cat.index(), sidebar_x + 13.0, cat_y + 8.0, 10.0, icon_color);
        let label_color = if active { a(palette.text) } else { a(palette.text_dim) };
        batch.text(atlas, cat.display_name(), sidebar_x + 30.0, cat_y + 9.0, label_color, active);
        let count = registry
            .features
            .iter()
            .filter(|f| f.category == cat && f.enabled)
            .count();
        if count > 0 {
            let text = count.to_string();
            let tw = HudBatch::text_width(atlas, &text);
            theme::rounded_rect(
                batch,
                atlas,
                sidebar_x + l.sidebar - tw - 16.0,
                cat_y + 8.0,
                tw + 8.0,
                11.0,
                4.0,
                a(if active { palette.accent } else { palette.card }),
            );
            batch.text(
                atlas,
                &text,
                sidebar_x + l.sidebar - tw - 12.0,
                cat_y + 9.0,
                a(if active { palette.header } else { palette.text_dim }),
                false,
            );
        }
        cat_y += row_h + 2.0;
    }

    // Footer: enabled count and the listening hint.
    let enabled = registry.features.iter().filter(|f| f.enabled).count();
    batch.text(
        atlas,
        &format!("{enabled} enabled"),
        sidebar_x + 10.0,
        wy + wh - 16.0,
        a(palette.text_muted),
        false,
    );

    // Resize handle.
    for i in 0..3 {
        batch.fill(
            atlas,
            wx + ww - 5.0 - i as f32 * 3.0,
            wy + wh - 4.0,
            2.0 + i as f32 * 3.0,
            1.5,
            a(palette.border),
        );
    }

    // Overlays.
    if let Some(dd) = &state.dropdown {
        draw_dropdown(batch, atlas, dd, palette, a);
    }
    if let Some(cp) = &state.color_picker {
        draw_color_picker(batch, atlas, cp, palette, a);
    }
    if state.listening.is_some() {
        let text = "Press a key…  (Esc cancels)";
        let tw = HudBatch::text_width(atlas, text);
        let bx = wx + (ww - tw) * 0.5 - 10.0;
        let by = wy + wh - 30.0;
        theme::rounded_rect(batch, atlas, bx, by, tw + 20.0, 16.0, 5.0, a(palette.accent_dim));
        batch.text(atlas, text, bx + 10.0, by + 4.0, a(palette.text), true);
    }
}

fn draw_card(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    state: &UiState,
    f: &Feature,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    palette: &Palette,
    a: impl Fn(u32) -> u32,
) {
    let base = if f.expanded {
        palette.card_open
    } else if f.enabled {
        theme::mix(palette.card, palette.accent, 0.10)
    } else {
        palette.card
    };
    theme::rounded_rect(batch, atlas, x, y, w, h, 6.0, a(base));
    if f.enabled {
        batch.fill(atlas, x, y + 4.0, 2.0, h - 8.0, a(palette.accent));
    }

    let name_color = if f.enabled { a(palette.text) } else { a(palette.text_dim) };
    batch.text(atlas, &truncate(&f.name, 26), x + 9.0, y + 7.0, name_color, f.enabled);

    // Bind pill.
    if f.bind != "None" {
        let label = pretty_key(&f.bind);
        let tw = HudBatch::text_width(atlas, &label);
        let bx = x + w - 74.0 - tw;
        if bx > x + 90.0 {
            theme::rounded_rect(batch, atlas, bx, y + 7.0, tw + 8.0, 12.0, 3.0, a(palette.accent_dim));
            batch.text(atlas, &label, bx + 4.0, y + 8.0, a(palette.accent), false);
        }
    }

    // Chevron.
    if !f.settings.is_empty() || matches!(f.id.as_str(), "configs" | "friends" | "gui_settings") {
        theme::chevron(
            batch,
            atlas,
            x + w - 64.0,
            y + 9.0,
            8.0,
            f.expanded,
            a(palette.text_muted),
        );
    }

    // Toggle.
    let tw = 28.0;
    let th = 14.0;
    let tx = x + w - tw - 8.0;
    let ty = y + 7.0;
    let track = if f.enabled { palette.accent } else { palette.toggle_off };
    theme::rounded_rect(batch, atlas, tx, ty, tw, th, th * 0.5, a(track));
    let knob_x = if f.enabled { tx + tw - th + 1.0 } else { tx + 1.0 };
    theme::rounded_rect(batch, atlas, knob_x, ty + 1.0, th - 2.0, th - 2.0, (th - 2.0) * 0.5, a(palette.text));

    if !f.expanded {
        return;
    }

    // Expanded body.
    batch.text(
        atlas,
        &truncate(&f.description, 78),
        x + 10.0,
        y + 24.0,
        a(palette.text_muted),
        false,
    );

    if f.id == "configs" {
        let buttons = ["Save", "Reload", "Reset"];
        let mut bx = x + 10.0;
        for (i, label) in buttons.iter().enumerate() {
            let bw = (w - 32.0) / 3.0;
            let col = if i == 2 { rgba(180, 60, 70, 230) } else { palette.card_hover };
            theme::rounded_rect(batch, atlas, bx, y + 36.0, bw, 18.0, 5.0, a(col));
            let tw = HudBatch::text_width(atlas, label);
            batch.text(atlas, label, bx + (bw - tw) * 0.5, y + 41.0, a(palette.text), false);
            bx += bw + 4.0;
        }
        return;
    }
    if f.id == "friends" {
        let mut row_y = y + 36.0;
        let list = state.friends_snapshot();
        if list.is_empty() {
            batch.text(atlas, "No friends yet.", x + 10.0, row_y, a(palette.text_muted), false);
        }
        for name in list.iter().take(4) {
            theme::rounded_rect(batch, atlas, x + 10.0, row_y - 2.0, w - 20.0, 16.0, 4.0, a(palette.card_hover));
            batch.text(atlas, &truncate(name, 24), x + 16.0, row_y + 2.0, a(palette.text), false);
            let xw = HudBatch::text_width(atlas, "×");
            batch.text(atlas, "×", x + w - 22.0 - xw, row_y + 2.0, a(palette.red), true);
            row_y += 18.0;
        }
        theme::rounded_rect(batch, atlas, x + 10.0, row_y, w - 20.0, 18.0, 5.0, a(palette.accent_dim));
        batch.text(atlas, "+ Add player under crosshair", x + 16.0, row_y + 5.0, a(palette.accent), false);
        return;
    }

    let mut row_y = y + 36.0;
    if f.id == "gui_settings" {
        // Palette presets.
        let mut sx = x + 10.0;
        for (_, c) in theme::ACCENT_PRESETS {
            theme::rounded_rect(batch, atlas, sx, row_y, 16.0, 14.0, 4.0, a(rgba(c[0], c[1], c[2], 255)));
            if c[..3] == state.accent_snapshot()[..3] {
                theme::rounded_rect(batch, atlas, sx + 5.0, row_y + 5.0, 6.0, 4.0, 2.0, a(palette.header));
            }
            sx += 19.0;
        }
        row_y += 22.0;
    }

    for s in &f.settings {
        theme::separator(batch, atlas, x + 10.0, row_y - 3.0, w - 20.0, a(palette.border_soft));
        batch.text(atlas, &truncate(&s.name, 22), x + 12.0, row_y + 2.0, a(palette.text_dim), false);
        match &s.value {
            SettingValue::Bool(v) => {
                let sw = 22.0;
                let sh = 12.0;
                let sx = x + w - sw - 12.0;
                theme::rounded_rect(batch, atlas, sx, row_y + 1.0, sw, sh, sh * 0.5, a(if *v { palette.accent } else { palette.toggle_off }));
                let kx = if *v { sx + sw - sh + 1.0 } else { sx + 1.0 };
                theme::rounded_rect(batch, atlas, kx, row_y + 2.0, sh - 2.0, sh - 2.0, (sh - 2.0) * 0.5, a(palette.text));
            }
            SettingValue::Enum { selected, options } => {
                let label = options.get(*selected).map_or("—", |o| o.as_str());
                let tw = HudBatch::text_width(atlas, label);
                let px = x + w - tw - 26.0;
                theme::rounded_rect(batch, atlas, px, row_y, tw + 16.0, 14.0, 4.0, a(palette.card_hover));
                batch.text(atlas, label, px + 6.0, row_y + 3.0, a(palette.accent), false);
                theme::chevron(batch, atlas, px + tw + 10.0, row_y + 4.0, 5.0, true, a(palette.text_muted));
            }
            SettingValue::Color(c) => {
                theme::rounded_rect(batch, atlas, x + w - 28.0, row_y, 16.0, 14.0, 4.0, a(rgba(c[0], c[1], c[2], c[3])));
                batch.fill(atlas, x + w - 28.0, row_y, 16.0, 1.0, a(rgba(255, 255, 255, 70)));
            }
            SettingValue::Keybind(k) => {
                let label = pretty_key(k);
                let tw = HudBatch::text_width(atlas, &label);
                let px = x + w - tw - 20.0;
                theme::rounded_rect(batch, atlas, px, row_y, tw + 12.0, 14.0, 4.0, a(palette.accent_dim));
                batch.text(atlas, &label, px + 6.0, row_y + 3.0, a(palette.accent), false);
            }
            SettingValue::Float { .. } | SettingValue::Range { .. } | SettingValue::Int { .. } => {
                if let Some((value, min, max, _)) = s.slider() {
                    let pct = ((value - min) / (max - min).max(0.001)).clamp(0.0, 1.0);
                    let track_x = x + 12.0;
                    let track_w = w - 24.0;
                    theme::rounded_rect(batch, atlas, track_x, row_y + 16.0, track_w, 4.0, 2.0, a(palette.track));
                    theme::rounded_rect(batch, atlas, track_x, row_y + 16.0, track_w * pct, 4.0, 2.0, a(palette.accent));
                    theme::rounded_rect(batch, atlas, track_x + track_w * pct - 3.0, row_y + 13.0, 6.0, 10.0, 3.0, a(palette.text));
                    let val = if matches!(s.value, SettingValue::Int { .. }) {
                        format!("{}", value as i32)
                    } else if (max - min) <= 1.5 {
                        format!("{value:.2}")
                    } else {
                        format!("{value:.1}")
                    };
                    let vw = HudBatch::text_width(atlas, &val);
                    batch.text(atlas, &val, x + w - vw - 12.0, row_y + 2.0, a(palette.accent), false);
                }
            }
        }
        row_y += 26.0;
    }
}

fn draw_dropdown(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    dd: &Dropdown,
    palette: &Palette,
    a: impl Fn(u32) -> u32,
) {
    let row_h = 16.0;
    let h = dd.options.len() as f32 * row_h + 8.0;
    theme::shadow(batch, atlas, dd.x, dd.y, dd.w, h, 6.0, rgba(0, 0, 0, 170));
    theme::rounded_rect(batch, atlas, dd.x, dd.y, dd.w, h, 6.0, a(palette.header));
    for (i, opt) in dd.options.iter().enumerate() {
        let y = dd.y + 4.0 + i as f32 * row_h;
        if i == dd.selected {
            theme::rounded_rect(batch, atlas, dd.x + 3.0, y, dd.w - 6.0, row_h, 4.0, a(palette.accent_dim));
        }
        batch.text(
            atlas,
            &truncate(opt, 26),
            dd.x + 8.0,
            y + 4.0,
            a(if i == dd.selected { palette.accent } else { palette.text_dim }),
            i == dd.selected,
        );
    }
}

fn draw_color_picker(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    cp: &ColorPicker,
    palette: &Palette,
    a: impl Fn(u32) -> u32,
) {
    theme::shadow(batch, atlas, cp.x, cp.y, cp.w, cp.h, 6.0, rgba(0, 0, 0, 170));
    theme::rounded_rect(batch, atlas, cp.x, cp.y, cp.w, cp.h, 6.0, a(palette.header));
    // 8x2 palette.
    let sw = (cp.w - 20.0) / 8.0;
    for i in 0..16 {
        let (r, g, b) = hue_grid(i);
        let x = cp.x + 8.0 + (i % 8) as f32 * sw;
        let y = cp.y + 8.0 + (i / 8) as f32 * (sw + 2.0);
        theme::rounded_rect(batch, atlas, x, y, sw - 2.0, sw - 2.0, 3.0, a(rgba(r, g, b, 255)));
        if cp.color[0] == r && cp.color[1] == g && cp.color[2] == b {
            batch.fill(atlas, x + 2.0, y + sw * 0.5, sw - 6.0, 1.5, a(rgba(10, 12, 18, 255)));
        }
    }
    // Alpha track.
    let track_y = cp.y + cp.h - 16.0;
    let track_x = cp.x + 8.0;
    let track_w = cp.w - 16.0;
    theme::rounded_rect(batch, atlas, track_x, track_y, track_w, 6.0, 3.0, a(palette.track));
    theme::rounded_rect(
        batch,
        atlas,
        track_x,
        track_y,
        track_w * (cp.color[3] as f32 / 255.0),
        6.0,
        3.0,
        a(rgba(cp.color[0], cp.color[1], cp.color[2], 255)),
    );
}

fn hue_grid(i: usize) -> (u8, u8, u8) {
    const GRID: [(u8, u8, u8); 16] = [
        (255, 255, 255),
        (190, 196, 210),
        (120, 128, 150),
        (54, 58, 74),
        (10, 12, 18),
        (255, 92, 140),
        (255, 86, 96),
        (255, 140, 66),
        (255, 186, 66),
        (255, 238, 88),
        (124, 92, 255),
        (64, 148, 255),
        (46, 214, 214),
        (58, 220, 150),
        (150, 92, 255),
        (220, 120, 255),
    ];
    GRID[i % GRID.len()]
}

impl UiState {
    fn friends_snapshot(&self) -> Vec<String> {
        self.cached_friends.clone()
    }
}

/// Extra state kept beside the UI state: the friends snapshot and the accent.
impl UiState {
    pub fn set_friends_cache(&mut self, friends: Vec<String>) {
        self.cached_friends = friends;
    }

    pub fn set_accent_cache(&mut self, accent: [u8; 4]) {
        self.cached_accent = accent;
    }

    fn accent_snapshot(&self) -> [u8; 4] {
        self.cached_accent
    }
}

/// Touch dispatch. Returns true when the menu consumed the event.
#[allow(clippy::too_many_arguments)]
pub fn handle_touch(
    phase: TouchPhase,
    point: [f32; 2],
    touch_id: u64,
    state: &mut UiState,
    registry: &mut FeatureRegistry,
    config: &mut ModConfig,
    _palette: &Palette,
    friends: &[String],
    atlas: &HudAtlas,
) -> bool {
    let [px, py] = point;
    let screen_w = state.screen_w.max(320.0);
    let screen_h = state.screen_h.max(240.0);
    let l = Layout::new(state, screen_w, screen_h);

    // --- Overlays first ---
    if let Some(dd) = state.dropdown.clone() {
        let row_h = 16.0;
        let h = dd.options.len() as f32 * row_h + 8.0;
        if px >= dd.x && px <= dd.x + dd.w && py >= dd.y && py <= dd.y + h {
            if phase == TouchPhase::Started {
                let idx = ((py - dd.y - 4.0) / row_h).floor().max(0.0) as usize;
                if idx < dd.options.len() {
                    if let Some(f) = registry.get_feature_mut(&dd.feature) {
                        if let Some(s) = f.setting_mut(&dd.key) {
                            if let SettingValue::Enum { selected, .. } = &mut s.value {
                                *selected = idx;
                            }
                        }
                    }
                    state.sound(SoundKind::Click);
                }
                state.dropdown = None;
                return true;
            }
        } else if phase == TouchPhase::Started {
            state.dropdown = None;
            return true;
        }
    }
    if let Some(cp) = state.color_picker.clone() {
        let inside = px >= cp.x && px <= cp.x + cp.w && py >= cp.y && py <= cp.y + cp.h;
        if phase == TouchPhase::Started && inside {
            let sw = (cp.w - 20.0) / 8.0;
            let gx = ((px - cp.x - 8.0) / sw).floor();
            let gy = ((py - cp.y - 8.0) / (sw + 2.0)).floor();
            if (0.0..8.0).contains(&gx) && (0.0..2.0).contains(&gy) {
                let (r, g, b) = hue_grid(gy as usize * 8 + gx as usize);
                set_color(registry, &cp.feature, &cp.key, [r, g, b, cp.color[3]]);
                state.sound(SoundKind::Click);
            } else if py >= cp.y + cp.h - 20.0 && py <= cp.y + cp.h - 6.0 {
                let pct = ((px - cp.x - 8.0) / (cp.w - 16.0)).clamp(0.0, 1.0);
                let mut c = cp.color;
                c[3] = (pct * 255.0) as u8;
                set_color(registry, &cp.feature, &cp.key, c);
            }
            return true;
        }
        if phase == TouchPhase::Started {
            state.color_picker = None;
            return true;
        }
    }

    // --- Floating button ---
    let bx = state.floating_btn_x;
    let by = state.floating_btn_y;
    let bw = 62.0;
    let bh = 22.0;
    if !state.open && px >= bx && px <= bx + bw && py >= by && py <= by + bh {
        match phase {
            TouchPhase::Started => {
                state.active_touch_id = Some(touch_id);
                state.dragging_button = true;
                state.drag_start_touch = [px, py];
                state.drag_start_button = [bx, by];
                return true;
            }
            TouchPhase::Moved if state.dragging_button => {
                state.floating_btn_x = (state.drag_start_button[0] + px - state.drag_start_touch[0]).max(0.0);
                state.floating_btn_y = (state.drag_start_button[1] + py - state.drag_start_touch[1]).max(0.0);
                return true;
            }
            TouchPhase::Ended | TouchPhase::Cancelled if state.dragging_button => {
                let moved = (px - state.drag_start_touch[0]).abs() + (py - state.drag_start_touch[1]).abs();
                state.dragging_button = false;
                state.active_touch_id = None;
                if moved < 8.0 {
                    state.open = true;
                    state.sound(SoundKind::Click);
                }
                return true;
            }
            _ => {}
        }
    }

    if !state.open {
        return false;
    }

    let wx = l.wx;
    let wy = l.wy;
    let ww = l.ww;
    let wh = l.wh;
    let in_window = px >= wx && px <= wx + ww && py >= wy && py <= wy + wh;

    match phase {
        TouchPhase::Started => {
            if !in_window {
                return false;
            }
            state.active_touch_id = Some(touch_id);

            // Resize handle.
            if px >= wx + ww - 16.0 && py >= wy + wh - 16.0 {
                state.resizing = true;
                state.resize_start_touch = [px, py];
                state.resize_start_size = [state.window_w, state.window_h];
                return true;
            }

            // Header: close, minimise, search, drag.
            if py <= wy + l.header {
                let close_x = wx + ww - 24.0;
                let min_x = close_x - 22.0;
                if px >= close_x {
                    state.open = false;
                    state.save_to_config(config);
                    config.save();
                    state.sound(SoundKind::Click);
                    return true;
                }
                if px >= min_x && px < close_x {
                    state.collapsed = !state.collapsed;
                    state.window_h = if state.collapsed { 30.0 } else { 360.0 };
                    return true;
                }
                let search_w = (ww * 0.32).clamp(90.0, 190.0);
                let search_x = wx + ww - search_w - 56.0;
                if px >= search_x && px <= search_x + search_w && py >= wy + 6.0 && py <= wy + 24.0 {
                    state.search_focus = true;
                    state.sound(SoundKind::Click);
                    return true;
                }
                state.search_focus = false;
                state.dragging_window = true;
                state.drag_start_touch = [px, py];
                state.drag_start_window = [wx, wy];
                return true;
            }
            if state.collapsed {
                return true;
            }

            // Sidebar.
            if px < wx + l.sidebar {
                let mut cat_y = wy + l.header + 6.0;
                for cat in FeatureCategory::ALL {
                    if py >= cat_y && py <= cat_y + 26.0 {
                        state.active_category = cat;
                        state.search_query.clear();
                        state.search_focus = false;
                        state.scroll_offset = 0.0;
                        state.sound(SoundKind::Click);
                        return true;
                    }
                    cat_y += 28.0;
                }
                return true;
            }

            // Content list.
            let list_x = wx + l.sidebar + 8.0;
            let list_y = wy + l.header + 6.0;
            let list_w = ww - l.sidebar - 16.0;
            let list_h = wh - l.header - 12.0;
            if px >= list_x && px <= list_x + list_w && py >= list_y && py <= list_y + list_h {
                let items = visible_indices(registry, state);
                let mut y = list_y - state.scroll_offset;
                for real in items {
                    let f = &registry.features[real];
                    let h = card_height(f);
                    if py >= y && py <= y + h {
                        let feature = registry.features[real].clone();
                        // Hover blip when the finger lands on a different card.
                        if state.hover.as_deref() != Some(feature.id.as_str()) {
                            state.hover = Some(feature.id.clone());
                            state.sound(SoundKind::Hover);
                        }
                        return card_touch(
                            state, registry, config, friends, atlas, real, &feature, px, py,
                            list_x, y, list_w, h,
                        );
                    }
                    y += h + 4.0;
                }
                // Empty space: start scrolling.
                state.dragging_scroll = true;
                state.scroll_start_touch = py;
                state.scroll_start_offset = state.scroll_offset;
                state.scroll_moved = false;
                return true;
            }
            return true;
        }
        TouchPhase::Moved => {
            if state.active_touch_id != Some(touch_id) {
                return in_window;
            }
            if state.dragging_window {
                state.window_x = (state.drag_start_window[0] + px - state.drag_start_touch[0]).max(0.0);
                state.window_y = (state.drag_start_window[1] + py - state.drag_start_touch[1]).max(0.0);
                return true;
            }
            if state.resizing {
                let dw = px - state.resize_start_touch[0];
                let dh = py - state.resize_start_touch[1];
                state.window_w = (state.resize_start_size[0] + dw).clamp(300.0, screen_w - 4.0);
                state.window_h = (state.resize_start_size[1] + dh).clamp(200.0, screen_h - 4.0);
                return true;
            }
            if state.dragging_scroll {
                let dy = py - state.scroll_start_touch;
                if dy.abs() > 4.0 {
                    state.scroll_moved = true;
                }
                let list_h = wh - l.header - 12.0;
                let max_scroll = (state.content_h - list_h).max(0.0);
                state.scroll_offset = (state.scroll_start_offset - dy).clamp(0.0, max_scroll);
                return true;
            }
            if let Some((fid, key)) = state.active_slider.clone() {
                if let Some(real) = registry.get_feature_mut(&fid) {
                    if let Some(s) = real.setting_mut(&key) {
                        if let Some((_, min, max, _)) = s.slider() {
                            let track_x = l.list_x + 12.0;
                            let track_w = l.list_w - 24.0;
                            let pct = ((px - track_x) / track_w).clamp(0.0, 1.0);
                            s.set_slider(min + pct * (max - min));
                        }
                    }
                }
                return true;
            }
            return true;
        }
        TouchPhase::Ended | TouchPhase::Cancelled => {
            let consumed = state.active_touch_id == Some(touch_id) || in_window;
            state.dragging_window = false;
            state.resizing = false;
            state.dragging_scroll = false;
            state.active_slider = None;
            state.active_touch_id = None;
            if consumed {
                state.save_to_config(config);
                config.save();
            }
            consumed
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn card_touch(
    state: &mut UiState,
    registry: &mut FeatureRegistry,
    config: &mut ModConfig,
    friends: &[String],
    atlas: &HudAtlas,
    _real: usize,
    f: &Feature,
    px: f32,
    py: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> bool {
    let toggle_x = x + w - 36.0;
    let chevron_x = x + w - 64.0;

    // Module toggle.
    if px >= toggle_x {
        if f.id == "panic" {
            state.pending_action = Some(UiAction::Panic);
            state.toast("Panic", "Every module disabled");
            state.sound(SoundKind::Click);
            return true;
        }
        let new_state = !f.enabled;
        if let Some(feat) = registry.get_feature_mut(&f.id) {
            feat.enabled = new_state;
            if new_state {
                state.toast("Enabled", &feat.name);
            } else {
                state.toast("Disabled", &feat.name);
            }
            state.sound(if new_state {
                SoundKind::ToggleOn
            } else {
                SoundKind::ToggleOff
            });
        }
        registry.save_to_config(config);
        config.save();
        return true;
    }

    // Chevron: expand.
    if px >= chevron_x - 8.0 && px <= chevron_x + 10.0 && py <= y + 22.0 {
        if let Some(feat) = registry.get_feature_mut(&f.id) {
            feat.expanded = !feat.expanded;
        }
        state.sound(SoundKind::Click);
        return true;
    }

    // Bind pill: listen for a module key.
    if f.bind != "None" {
        let label = pretty_key(&f.bind);
        let tw = HudBatch::text_width(atlas, &label);
        let bx = x + w - 74.0 - tw;
        if px >= bx && px <= bx + tw + 8.0 && py <= y + 22.0 {
            state.listening = Some(BindTarget::Module(f.id.clone()));
            state.sound(SoundKind::Click);
            return true;
        }
    }

    if !f.expanded {
        // A tap anywhere else on the card toggles it, as in CorzClient.
        let new_state = !f.enabled;
        if let Some(feat) = registry.get_feature_mut(&f.id) {
            feat.enabled = new_state;
            state.toast(if new_state { "Enabled" } else { "Disabled" }, &feat.name);
        }
        state.sound(if new_state {
            SoundKind::ToggleOn
        } else {
            SoundKind::ToggleOff
        });
        registry.save_to_config(config);
        config.save();
        return true;
    }

    // Special panels.
    if f.id == "configs" {
        let bw = (w - 32.0) / 3.0;
        if py >= y + 36.0 && py <= y + 54.0 {
            let idx = ((px - x - 10.0) / (bw + 4.0)).floor().max(0.0) as usize;
            state.pending_action = Some(match idx {
                0 => UiAction::SaveConfig,
                1 => UiAction::ReloadConfig,
                _ => UiAction::ResetConfig,
            });
            state.sound(SoundKind::Click);
        }
        return true;
    }
    if f.id == "friends" {
        let mut row_y = y + 36.0;
        for name in friends.iter().take(4) {
            if py >= row_y - 2.0 && py <= row_y + 14.0 {
                if px >= x + w - 30.0 {
                    state.pending_action = Some(UiAction::RemoveFriend(name.clone()));
                    state.sound(SoundKind::Click);
                }
                return true;
            }
            row_y += 18.0;
        }
        if py >= row_y && py <= row_y + 18.0 {
            state.pending_action = Some(UiAction::AddFriend);
            state.sound(SoundKind::Click);
        }
        return true;
    }

    let mut row_y = y + 36.0;
    if f.id == "gui_settings" {
        let mut sx = x + 10.0;
        for (_, c) in theme::ACCENT_PRESETS {
            if py >= row_y && py <= row_y + 14.0 && px >= sx && px <= sx + 16.0 {
                state.pending_action = Some(UiAction::SetAccent(c));
                state.sound(SoundKind::Click);
                return true;
            }
            sx += 19.0;
        }
        row_y += 22.0;
    }

    for s in &f.settings {
        if py >= row_y && py <= row_y + 24.0 {
            match &s.value {
                SettingValue::Bool(_) => {
                    if let Some(feat) = registry.get_feature_mut(&f.id) {
                        if let Some(set) = feat.setting_mut(&s.key) {
                            if let SettingValue::Bool(v) = &mut set.value {
                                *v = !*v;
                                state.sound(SoundKind::Click);
                            }
                        }
                    }
                }
                SettingValue::Enum { selected, options } => {
                    let label = options.get(*selected).map_or("", |o| o.as_str());
                    let tw = HudBatch::text_width(atlas, label);
                    let px0 = x + w - tw - 26.0;
                    if px >= px0 {
                        state.dropdown = Some(Dropdown {
                            feature: f.id.clone(),
                            key: s.key.clone(),
                            x: px0.min(x + w - 110.0),
                            y: row_y + 16.0,
                            w: 108.0,
                            options: options.clone(),
                            selected: *selected,
                        });
                    }
                    state.sound(SoundKind::Click);
                }
                SettingValue::Color(c) => {
                    let pw = 150.0;
                    state.color_picker = Some(ColorPicker {
                        feature: f.id.clone(),
                        key: s.key.clone(),
                        x: (x + w - pw - 8.0).max(4.0),
                        y: row_y + 18.0,
                        w: pw,
                        h: 84.0,
                        color: *c,
                    });
                    state.sound(SoundKind::Click);
                }
                SettingValue::Keybind(_) => {
                    state.listening = Some(BindTarget::Setting(f.id.clone(), s.key.clone()));
                    state.sound(SoundKind::Click);
                }
                SettingValue::Float { .. } | SettingValue::Range { .. } | SettingValue::Int { .. } => {
                    if let Some((_, min, max, _)) = s.slider() {
                        let track_x = x + 12.0;
                        let track_w = w - 24.0;
                        let pct = ((px - track_x) / track_w).clamp(0.0, 1.0);
                        if let Some(feat) = registry.get_feature_mut(&f.id) {
                            if let Some(set) = feat.setting_mut(&s.key) {
                                set.set_slider(min + pct * (max - min));
                            }
                        }
                        state.active_slider = Some((f.id.clone(), s.key.clone()));
                    }
                }
            }
            registry.save_to_config(config);
            config.save();
            return true;
        }
        row_y += 26.0;
    }
    let _ = h;
    true
}

fn set_color(registry: &mut FeatureRegistry, feature: &str, key: &str, color: [u8; 4]) {
    if let Some(f) = registry.get_feature_mut(feature) {
        if let Some(s) = f.setting_mut(key) {
            s.value = SettingValue::Color(color);
        }
    }
}

/// Draws toasts. Kept here so the menu owns one screen-space pass.
pub fn draw_notifications(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    notifications: &Notifications,
    palette: &Palette,
    anchor: usize,
    animations: bool,
) {
    notifications.draw(
        batch,
        atlas,
        palette,
        anchor,
        batch.width,
        batch.height,
        animations,
    );
}
