//! Corz-style theme: palette, rounded rectangles, glow and vector icons.
//!
//! Everything is drawn with `HudBatch` primitives, so the interface adds no
//! textures and costs no extra atlas memory on a 4 GB device.

use horizon_client_render::hud::{HudAtlas, HudBatch, rgba};

pub const ACCENT_PRESETS: [(&str, [u8; 4]); 8] = [
    ("Violet", [124, 92, 255, 255]),
    ("Azure", [64, 148, 255, 255]),
    ("Cyan", [46, 214, 214, 255]),
    ("Mint", [58, 220, 150, 255]),
    ("Amber", [255, 178, 58, 255]),
    ("Rose", [255, 92, 140, 255]),
    ("Crimson", [255, 74, 74, 255]),
    ("Slate", [170, 180, 200, 255]),
];

#[derive(Clone, Copy)]
pub struct Palette {
    pub accent: u32,
    pub accent_soft: u32,
    pub accent_dim: u32,
    pub window: u32,
    pub header: u32,
    pub sidebar: u32,
    pub card: u32,
    pub card_hover: u32,
    pub card_open: u32,
    pub border: u32,
    pub border_soft: u32,
    pub text: u32,
    pub text_dim: u32,
    pub text_muted: u32,
    pub toggle_off: u32,
    pub track: u32,
    pub green: u32,
    pub red: u32,
    pub amber: u32,
    pub shadow: u32,
}

pub fn palette(accent: [u8; 4], opacity: f32) -> Palette {
    let a = (opacity.clamp(0.55, 0.98) * 255.0) as u8;
    Palette {
        accent: rgba(accent[0], accent[1], accent[2], 255),
        accent_soft: rgba(accent[0], accent[1], accent[2], 90),
        accent_dim: rgba(accent[0], accent[1], accent[2], 36),
        window: rgba(13, 15, 22, a),
        header: rgba(20, 23, 34, 255),
        sidebar: rgba(16, 18, 27, 250),
        card: rgba(27, 30, 43, 232),
        card_hover: rgba(36, 40, 56, 240),
        card_open: rgba(23, 26, 38, 245),
        border: rgba(52, 58, 78, 190),
        border_soft: rgba(44, 49, 66, 120),
        text: rgba(246, 248, 255, 255),
        text_dim: rgba(168, 176, 198, 255),
        text_muted: rgba(120, 128, 150, 255),
        toggle_off: rgba(52, 57, 74, 255),
        track: rgba(30, 34, 48, 255),
        green: rgba(58, 220, 150, 255),
        red: rgba(255, 86, 96, 255),
        amber: rgba(255, 186, 66, 255),
        shadow: rgba(0, 0, 0, 120),
    }
}

/// Blend two packed colours (alpha ignored, 0..1).
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |shift: u32| {
        let x = ((a >> shift) & 0xff) as f32;
        let y = ((b >> shift) & 0xff) as f32;
        ((x + (y - x) * t) as u32) & 0xff
    };
    let alpha = ((a >> 24) & 0xff) as f32;
    let alpha_b = ((b >> 24) & 0xff) as f32;
    let out_a = (alpha + (alpha_b - alpha) * t) as u32 & 0xff;
    out_a << 24 | ch(16) << 16 | ch(8) << 8 | ch(0)
}

pub fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color & 0x00ff_ffff) | (u32::from(alpha) << 24)
}

pub fn scale_alpha(color: u32, factor: f32) -> u32 {
    let a = ((color >> 24) & 0xff) as f32 * factor.clamp(0.0, 1.0);
    with_alpha(color, a as u8)
}

fn tri(batch: &mut HudBatch, atlas: &HudAtlas, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: u32) {
    batch.fill_quad(atlas, [a, b, c, c], color);
}

/// A rounded rectangle built from a cross plus four quarter-circle fans.
pub fn rounded_rect(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: u32,
) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let r = radius.min(w * 0.5).min(h * 0.5).max(0.0);
    if r < 0.5 {
        batch.fill(atlas, x, y, w, h, color);
        return;
    }
    batch.fill(atlas, x + r, y, w - 2.0 * r, h, color);
    batch.fill(atlas, x, y + r, r, h - 2.0 * r, color);
    batch.fill(atlas, x + w - r, y + r, r, h - 2.0 * r, color);

    const STEPS: usize = 4;
    let centers = [
        (x + r, y + r, std::f32::consts::PI, 1.5 * std::f32::consts::PI),
        (
            x + w - r,
            y + r,
            1.5 * std::f32::consts::PI,
            std::f32::consts::TAU,
        ),
        (
            x + w - r,
            y + h - r,
            0.0,
            0.5 * std::f32::consts::PI,
        ),
        (x + r, y + h - r, 0.5 * std::f32::consts::PI, std::f32::consts::PI),
    ];
    for (cx, cy, start, end) in centers {
        for i in 0..STEPS {
            let a0 = start + (end - start) * i as f32 / STEPS as f32;
            let a1 = start + (end - start) * (i + 1) as f32 / STEPS as f32;
            let p0 = [cx + a0.cos() * r, cy + a0.sin() * r];
            let p1 = [cx + a1.cos() * r, cy + a1.sin() * r];
            tri(batch, atlas, [cx, cy], p0, p1, color);
        }
    }
}

/// A soft drop shadow made of a few expanding translucent rounded rects.
pub fn shadow(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: u32,
) {
    for i in 0..4 {
        let s = 2.0 + i as f32 * 2.5;
        let a = ((color >> 24) & 0xff) as f32 * (0.30 - i as f32 * 0.065);
        rounded_rect(
            batch,
            atlas,
            x - s,
            y - s + 1.5,
            w + s * 2.0,
            h + s * 2.0,
            radius + s,
            with_alpha(color, a.max(0.0) as u8),
        );
    }
}

/// Animated ambient glow: a wide, very transparent halo that breathes.
pub fn glow(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: u32,
    time: f32,
    strength: f32,
) {
    let pulse = 0.5 + 0.5 * (time * 1.6).sin();
    for i in 0..3 {
        let s = 6.0 + i as f32 * 7.0;
        let a = (10.0 + pulse * 10.0) * strength * (1.0 - i as f32 * 0.28);
        rounded_rect(
            batch,
            atlas,
            x - s,
            y - s,
            w + s * 2.0,
            h + s * 2.0,
            18.0 + s,
            with_alpha(color, a as u8),
        );
    }
}

/// A one-pixel-ish separator.
pub fn separator(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, w: f32, color: u32) {
    batch.fill(atlas, x, y, w, 1.0, color);
}

/// Category glyphs drawn from primitives. `i` is the category index.
pub fn category_icon(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    i: usize,
    x: f32,
    y: f32,
    s: f32,
    color: u32,
) {
    let t = (s * 0.16).max(1.0);
    match i {
        0 => {
            // Sword: blade + crossguard.
            tri(batch, atlas, [x + s * 0.18, y + s * 0.82], [x + s * 0.40, y + s * 0.88], [x + s * 0.86, y + s * 0.18], color);
            tri(batch, atlas, [x + s * 0.18, y + s * 0.82], [x + s * 0.86, y + s * 0.18], [x + s * 0.64, y + s * 0.10], color);
            batch.fill(atlas, x + s * 0.12, y + s * 0.62, s * 0.34, t, color);
            batch.fill(atlas, x + s * 0.10, y + s * 0.56, t, s * 0.26, color);
        }
        1 => {
            // Eye: two arcs around a pupil.
            batch.fill(atlas, x + s * 0.10, y + s * 0.42, s * 0.80, t, color);
            batch.fill(atlas, x + s * 0.10, y + s * 0.42, t, s * 0.16, color);
            batch.fill(atlas, x + s * 0.10, y + s * 0.56, t, s * 0.16, color);
            batch.fill(atlas, x + s * 0.80, y + s * 0.42, t, s * 0.16, color);
            batch.fill(atlas, x + s * 0.80, y + s * 0.56, t, s * 0.16, color);
            rounded_rect(batch, atlas, x + s * 0.38, y + s * 0.34, s * 0.24, s * 0.24, s * 0.12, color);
        }
        2 => {
            // Double chevron.
            for k in 0..2 {
                let o = k as f32 * s * 0.24;
                tri(batch, atlas, [x + s * 0.16 + o, y + s * 0.22], [x + s * 0.50 + o, y + s * 0.50], [x + s * 0.16 + o, y + s * 0.78], color);
            }
        }
        3 => {
            // Globe.
            rounded_rect(batch, atlas, x + s * 0.12, y + s * 0.12, s * 0.76, s * 0.76, s * 0.38, color);
            batch.fill(atlas, x + s * 0.12, y + s * 0.46, s * 0.76, t, rgba(20, 22, 32, 255));
            batch.fill(atlas, x + s * 0.44, y + s * 0.12, t, s * 0.76, rgba(20, 22, 32, 255));
        }
        4 => {
            // Sparkle.
            tri(batch, atlas, [x + s * 0.48, y + s * 0.08], [x + s * 0.58, y + s * 0.44], [x + s * 0.88, y + s * 0.52], color);
            tri(batch, atlas, [x + s * 0.48, y + s * 0.08], [x + s * 0.38, y + s * 0.44], [x + s * 0.08, y + s * 0.52], color);
            tri(batch, atlas, [x + s * 0.48, y + s * 0.94], [x + s * 0.58, y + s * 0.58], [x + s * 0.88, y + s * 0.50], color);
            tri(batch, atlas, [x + s * 0.48, y + s * 0.94], [x + s * 0.38, y + s * 0.58], [x + s * 0.08, y + s * 0.50], color);
        }
        _ => {
            // Gear: ring + teeth.
            rounded_rect(batch, atlas, x + s * 0.22, y + s * 0.22, s * 0.56, s * 0.56, s * 0.28, color);
            rounded_rect(batch, atlas, x + s * 0.40, y + s * 0.40, s * 0.20, s * 0.20, s * 0.10, rgba(20, 22, 32, 255));
            for k in 0..4 {
                let (dx, dy) = match k {
                    0 => (s * 0.44, s * 0.02),
                    1 => (s * 0.44, s * 0.86),
                    2 => (s * 0.02, s * 0.44),
                    _ => (s * 0.86, s * 0.44),
                };
                batch.fill(atlas, x + dx, y + dy, s * 0.12, s * 0.12, color);
            }
        }
    }
}

/// A small check mark.
pub fn check(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, s: f32, color: u32) {
    let t = (s * 0.18).max(1.0);
    tri(batch, atlas, [x, y + s * 0.45], [x + s * 0.18, y + s * 0.28], [x + s * 0.42, y + s * 0.62], color);
    tri(batch, atlas, [x, y + s * 0.45], [x + s * 0.42, y + s * 0.62], [x + s * 0.42, y + s * 0.82], color);
    tri(batch, atlas, [x + s * 0.30, y + s * 0.60], [x + s * 0.92, y - s * 0.02], [x + s * 1.02, y + s * 0.18], color);
    tri(batch, atlas, [x + s * 0.30, y + s * 0.60], [x + s * 1.02, y + s * 0.18], [x + s * 0.40, y + s * 0.78], color);
    let _ = t;
}

/// A chevron pointing right (`open` false) or down (`open` true).
pub fn chevron(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    s: f32,
    open: bool,
    color: u32,
) {
    if open {
        tri(batch, atlas, [x, y + s * 0.30], [x + s * 0.50, y + s * 0.70], [x + s, y + s * 0.30], color);
        tri(batch, atlas, [x, y + s * 0.30], [x + s, y + s * 0.30], [x + s * 0.50, y + s * 0.46], color);
    } else {
        tri(batch, atlas, [x + s * 0.30, y], [x + s * 0.70, y + s * 0.50], [x + s * 0.30, y + s], color);
        tri(batch, atlas, [x + s * 0.30, y], [x + s * 0.46, y + s * 0.50], [x + s * 0.30, y + s], color);
    }
}

/// A tiny lock, for a module that cannot run.
pub fn lock(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, s: f32, color: u32) {
    rounded_rect(batch, atlas, x, y + s * 0.42, s, s * 0.58, s * 0.10, color);
    batch.fill(atlas, x + s * 0.18, y + s * 0.12, s * 0.64, s * 0.36, color);
    batch.fill(atlas, x + s * 0.34, y + s * 0.20, s * 0.32, s * 0.30, rgba(20, 22, 32, 255));
}
