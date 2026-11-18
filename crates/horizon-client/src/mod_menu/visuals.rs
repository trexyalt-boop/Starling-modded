//! Visual rendering module: ESP bounding boxes, nametags, health indicators,
//! distance labels, tracers, coordinates HUD, and target info HUD.

use super::feature::FeatureRegistry;
use horizon_client_render::hud::{HudAtlas, HudBatch, rgba};
use horizon_core::geometry::Aabb;

/// Projects a 3D world coordinate to 2D GUI coordinates on the HUD.
pub fn project_point(
    world_pos: [f64; 3],
    cam_pos: [f64; 3],
    view_proj: &glam::Mat4,
    screen_w: f32,
    screen_h: f32,
) -> Option<[f32; 2]> {
    let dx = (world_pos[0] - cam_pos[0]) as f32;
    let dy = (world_pos[1] - cam_pos[1]) as f32;
    let dz = (world_pos[2] - cam_pos[2]) as f32;

    let clip = *view_proj * glam::Vec4::new(dx, dy, dz, 1.0);
    // Point behind or at near clipping plane
    if clip.w <= 0.05 {
        return None;
    }

    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;

    // Discard points excessively far outside screen bounds
    if ndc_x < -2.5 || ndc_x > 2.5 || ndc_y < -2.5 || ndc_y > 2.5 {
        return None;
    }

    let screen_x = (ndc_x + 1.0) * 0.5 * screen_w;
    let screen_y = (1.0 - ndc_y) * 0.5 * screen_h;
    Some([screen_x, screen_y])
}

/// Projects a 3D AABB into a 2D bounding box on the screen.
pub fn project_aabb(
    bounds: Aabb,
    cam_pos: [f64; 3],
    view_proj: &glam::Mat4,
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], [f32; 2])> {
    let min = bounds.min_position();
    let max = bounds.max_position();
    let corners = [
        [min.x, min.y, min.z],
        [max.x, min.y, min.z],
        [min.x, max.y, min.z],
        [max.x, max.y, min.z],
        [min.x, min.y, max.z],
        [max.x, min.y, max.z],
        [min.x, max.y, max.z],
        [max.x, max.y, max.z],
    ];

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut in_front = 0;

    for corner in corners {
        if let Some([sx, sy]) = project_point(corner, cam_pos, view_proj, screen_w, screen_h) {
            min_x = min_x.min(sx);
            min_y = min_y.min(sy);
            max_x = max_x.max(sx);
            max_y = max_y.max(sy);
            in_front += 1;
        }
    }

    // Require at least two projected corners to form a bounding box
    if in_front >= 2 && max_x > min_x && max_y > min_y {
        Some(([min_x, min_y], [max_x, max_y]))
    } else {
        None
    }
}

/// Renders a bordered rectangular outline on the HUD.
pub fn draw_rect_outline(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    thickness: f32,
    color: u32,
) {
    let shadow = rgba(0, 0, 0, 180);
    // Outer shadow for high contrast
    batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x - 1.0, y + h - thickness - 1.0, w + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x - 1.0, y - 1.0, thickness + 2.0, h + 2.0, shadow);
    batch.fill(atlas, x + w - thickness - 1.0, y - 1.0, thickness + 2.0, h + 2.0, shadow);

    // Main line
    batch.fill(atlas, x, y, w, thickness, color);
    batch.fill(atlas, x, y + h - thickness, w, thickness, color);
    batch.fill(atlas, x, y, thickness, h, color);
    batch.fill(atlas, x + w - thickness, y, thickness, h, color);
}

/// Renders corner bracket outlines.
pub fn draw_corner_box(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    thickness: f32,
    color: u32,
) {
    let len = (w.min(h) * 0.25).max(4.0);
    let shadow = rgba(0, 0, 0, 180);

    // Top-left
    batch.fill(atlas, x - 1.0, y - 1.0, len + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x - 1.0, y - 1.0, thickness + 2.0, len + 2.0, shadow);
    batch.fill(atlas, x, y, len, thickness, color);
    batch.fill(atlas, x, y, thickness, len, color);

    // Top-right
    batch.fill(atlas, x + w - len - 1.0, y - 1.0, len + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x + w - thickness - 1.0, y - 1.0, thickness + 2.0, len + 2.0, shadow);
    batch.fill(atlas, x + w - len, y, len, thickness, color);
    batch.fill(atlas, x + w - thickness, y, thickness, len, color);

    // Bottom-left
    batch.fill(atlas, x - 1.0, y + h - thickness - 1.0, len + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x - 1.0, y + h - len - 1.0, thickness + 2.0, len + 2.0, shadow);
    batch.fill(atlas, x, y + h - thickness, len, thickness, color);
    batch.fill(atlas, x, y + h - len, thickness, len, color);

    // Bottom-right
    batch.fill(atlas, x + w - len - 1.0, y + h - thickness - 1.0, len + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x + w - thickness - 1.0, y + h - len - 1.0, thickness + 2.0, len + 2.0, shadow);
    batch.fill(atlas, x + w - len, y + h - thickness, len, thickness, color);
    batch.fill(atlas, x + w - thickness, y + h - len, thickness, len, color);
}

/// Renders a vertical health bar on the left of the bounding box.
pub fn draw_health_bar(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    box_x: f32,
    box_y: f32,
    box_h: f32,
    health: f32,
    max_health: f32,
) {
    let bar_w = 3.0;
    let bar_x = box_x - bar_w - 3.0;
    let bar_y = box_y;
    let bar_h = box_h.max(6.0);

    // Background track
    batch.fill(atlas, bar_x - 1.0, bar_y - 1.0, bar_w + 2.0, bar_h + 2.0, rgba(0, 0, 0, 180));
    batch.fill(atlas, bar_x, bar_y, bar_w, bar_h, rgba(30, 30, 35, 200));

    // Filled health portion
    let pct = (health / max_health.max(1.0)).clamp(0.0, 1.0);
    let fill_h = bar_h * pct;
    let fill_y = bar_y + (bar_h - fill_h);

    // Dynamic color gradient (green -> yellow -> red)
    let color = if pct > 0.6 {
        rgba(46, 204, 113, 255) // Green
    } else if pct > 0.3 {
        rgba(241, 196, 15, 255) // Yellow
    } else {
        rgba(231, 76, 60, 255) // Red
    };

    batch.fill(atlas, bar_x, fill_y, bar_w, fill_h, color);
}

/// Renders target information HUD card.
pub fn draw_target_hud(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    target_name: &str,
    target_kind: &str,
    health: f32,
    max_health: f32,
    distance: f32,
) {
    let feat = match registry.get_feature("target_hud") {
        Some(f) if f.enabled => f,
        _ => return,
    };

    let base_x = feat.get_float_setting("hud_x").max(10.0);
    let base_y = feat.get_float_setting("hud_y").max(30.0);
    let card_w = 170.0;
    let card_h = 48.0;

    // Card background
    batch.fill(atlas, base_x, base_y, card_w, card_h, rgba(16, 18, 26, 225));
    batch.fill(atlas, base_x, base_y, card_w, 2.0, rgba(64, 128, 255, 255)); // Top accent line
    batch.fill(atlas, base_x, base_y + card_h - 1.0, card_w, 1.0, rgba(40, 44, 60, 180));
    batch.fill(atlas, base_x, base_y, 1.0, card_h, rgba(40, 44, 60, 180));
    batch.fill(atlas, base_x + card_w - 1.0, base_y, 1.0, card_h, rgba(40, 44, 60, 180));

    // Entity name
    let clean_name = if !target_name.is_empty() {
        target_name
    } else {
        target_kind.trim_start_matches("minecraft:")
    };
    batch.text(atlas, clean_name, base_x + 8.0, base_y + 6.0, rgba(255, 255, 255, 255), true);

    // Distance badge
    let dist_str = format!("{distance:.1}m");
    let dist_w = HudBatch::text_width(atlas, &dist_str);
    batch.fill(atlas, base_x + card_w - dist_w - 12.0, base_y + 5.0, dist_w + 6.0, 11.0, rgba(35, 40, 55, 220));
    batch.text(atlas, &dist_str, base_x + card_w - dist_w - 9.0, base_y + 6.0, rgba(200, 220, 255, 255), false);

    // Health text and bar
    let pct = (health / max_health.max(1.0)).clamp(0.0, 1.0);
    let hp_str = format!("HP: {health:.0}/{max_health:.0}");
    batch.text(atlas, &hp_str, base_x + 8.0, base_y + 20.0, rgba(170, 180, 200, 255), false);

    if feat.get_bool_setting("show_hp_bar") {
        let bar_x = base_x + 8.0;
        let bar_y = base_y + 32.0;
        let bar_w = card_w - 16.0;
        let bar_h = 7.0;

        batch.fill(atlas, bar_x, bar_y, bar_w, bar_h, rgba(25, 28, 38, 255));
        let hp_color = if pct > 0.5 {
            rgba(46, 204, 113, 255)
        } else if pct > 0.25 {
            rgba(241, 196, 15, 255)
        } else {
            rgba(231, 76, 60, 255)
        };
        batch.fill(atlas, bar_x, bar_y, bar_w * pct, bar_h, hp_color);
    }
}

/// Renders on-screen coordinates and heading HUD.
pub fn draw_coords_hud(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    pos: [f64; 3],
    yaw: f32,
    pitch: f32,
    biome_name: &str,
    fps: f32,
) {
    if !registry.is_enabled("coords_hud") {
        return;
    }

    let x = 6.0;
    let mut y = 24.0;
    let line_h = 10.0;

    let coords_line = format!("XYZ: {:.1} / {:.1} / {:.1}", pos[0], pos[1], pos[2]);
    let facing_line = format!("Facing: {:.1}° / {:.1}°", yaw, pitch);

    let mut lines = vec![coords_line, facing_line];
    if let Some(f) = registry.get_feature("coords_hud") {
        if f.get_bool_setting("show_biome") && !biome_name.is_empty() {
            lines.push(format!("Biome: {}", biome_name.trim_start_matches("minecraft:")));
        }
        if f.get_bool_setting("show_fps") {
            lines.push(format!("FPS: {:.0}", fps));
        }
    }

    let max_w = lines.iter().map(|l| HudBatch::text_width(atlas, l)).fold(0.0f32, f32::max);

    // Translucent dark backing for readability
    batch.fill(atlas, x - 2.0, y - 2.0, max_w + 6.0, lines.len() as f32 * line_h + 3.0, rgba(0, 0, 0, 120));

    for line in lines {
        batch.text(atlas, &line, x, y, rgba(230, 235, 245, 255), true);
        y += line_h;
    }
}

/// Renders the client watermark brand.
pub fn draw_watermark(batch: &mut HudBatch, atlas: &HudAtlas, registry: &FeatureRegistry) {
    if !registry.is_enabled("watermark") {
        return;
    }
    let brand = "STARLING v2.0 [MODS]";
    let w = HudBatch::text_width(atlas, brand);
    batch.fill(atlas, 4.0, 4.0, w + 8.0, 13.0, rgba(16, 20, 30, 200));
    batch.fill(atlas, 4.0, 4.0, 2.0, 13.0, rgba(64, 128, 255, 255));
    batch.text(atlas, brand, 8.0, 6.0, rgba(240, 245, 255, 255), true);
}
