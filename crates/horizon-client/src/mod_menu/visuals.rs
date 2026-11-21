//! World visuals and HUD widgets for the Corz menu: ESP, tracers, storage,
//! waypoints, trails, jump rings, target HUD, array list, crosshair and the
//! small status cards.

use super::config::SavedWaypoint;
use super::feature::FeatureRegistry;
use super::theme;
use horizon_client_render::hud::{HudAtlas, HudBatch, rgba};
use horizon_core::geometry::Aabb;
use std::time::Instant;

/// Everything the HUD needs from the game for one frame.
pub struct HudData<'a> {
    pub cam_pos: [f64; 3],
    pub cam_view: [f32; 16],
    pub cam_proj: [f32; 16],
    pub cam_fov: f32,
    pub player_pos: [f64; 3],
    pub player_yaw: f32,
    pub player_pitch: f32,
    pub biome: &'a str,
    pub dimension: &'a str,
    pub fps: f32,
    pub mobs: &'a [(usize, u32, &'a str, Option<&'a str>, [f64; 3], Aabb, f32, f32)],
    pub storage: &'a [(u8, [f64; 3])],
    pub waypoints: &'a [SavedWaypoint],
    pub trails: &'a [[f64; 3]],
    pub jump_rings: &'a [([f64; 3], Instant)],
    pub totems: usize,
    pub offhand_totem: bool,
    pub target_block: Option<([i32; 3], Aabb)>,
    pub falling: bool,
    pub attack_scale: f32,
}

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
    if clip.w <= 0.05 {
        return None;
    }
    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;
    if ndc_x < -2.5 || ndc_x > 2.5 || ndc_y < -2.5 || ndc_y > 2.5 {
        return None;
    }
    Some([
        (ndc_x + 1.0) * 0.5 * screen_w,
        (1.0 - ndc_y) * 0.5 * screen_h,
    ])
}

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

    if in_front >= 2 && max_x > min_x && max_y > min_y {
        Some(([min_x, min_y], [max_x, max_y]))
    } else {
        None
    }
}

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
    let shadow = rgba(0, 0, 0, 150);
    batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x - 1.0, y + h - thickness - 1.0, w + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x - 1.0, y - 1.0, thickness + 2.0, h + 2.0, shadow);
    batch.fill(atlas, x + w - thickness - 1.0, y - 1.0, thickness + 2.0, h + 2.0, shadow);

    batch.fill(atlas, x, y, w, thickness, color);
    batch.fill(atlas, x, y + h - thickness, w, thickness, color);
    batch.fill(atlas, x, y, thickness, h, color);
    batch.fill(atlas, x + w - thickness, y, thickness, h, color);
}

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
    let len = (w.min(h) * 0.28).max(4.0);
    let shadow = rgba(0, 0, 0, 150);
    // Top-left corner, with its shadow.
    batch.fill(atlas, x - 1.0, y - 1.0, len + 2.0, thickness + 2.0, shadow);
    batch.fill(atlas, x - 1.0, y - 1.0, thickness + 2.0, len + 2.0, shadow);
    batch.fill(atlas, x, y, len, thickness, color);
    batch.fill(atlas, x, y, thickness, len, color);
    batch.fill(atlas, x + w - len, y, len, thickness, color);
    batch.fill(atlas, x + w - thickness, y, thickness, len, color);
    batch.fill(atlas, x, y + h - thickness, len, thickness, color);
    batch.fill(atlas, x, y + h - len, thickness, len, color);
    batch.fill(atlas, x + w - len, y + h - thickness, len, thickness, color);
    batch.fill(atlas, x + w - thickness, y + h - len, thickness, len, color);
}

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
    let bar_h = box_h.max(6.0);

    batch.fill(atlas, bar_x - 1.0, bar_y_pad(box_y) - 1.0, bar_w + 2.0, bar_h + 2.0, rgba(0, 0, 0, 170));
    batch.fill(atlas, bar_x, box_y, bar_w, bar_h, rgba(30, 30, 38, 210));

    let pct = (health / max_health.max(1.0)).clamp(0.0, 1.0);
    let fill_h = bar_h * pct;
    let color = if pct > 0.6 {
        rgba(58, 220, 150, 255)
    } else if pct > 0.3 {
        rgba(255, 186, 66, 255)
    } else {
        rgba(255, 86, 96, 255)
    };
    batch.fill(atlas, bar_x, box_y + (bar_h - fill_h), bar_w, fill_h, color);
}

fn bar_y_pad(y: f32) -> f32 {
    y
}

fn health_color(health: f32, max: f32) -> u32 {
    let pct = (health / max.max(1.0)).clamp(0.0, 1.0);
    if pct > 0.6 {
        rgba(58, 220, 150, 255)
    } else if pct > 0.3 {
        rgba(255, 186, 66, 255)
    } else {
        rgba(255, 86, 96, 255)
    }
}

fn line(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    color: u32,
) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let nx = -dy / len * width * 0.5;
    let ny = dx / len * width * 0.5;
    batch.fill_quad(
        atlas,
        [
            [a[0] + nx, a[1] + ny],
            [b[0] + nx, b[1] + ny],
            [b[0] - nx, b[1] - ny],
            [a[0] - nx, a[1] - ny],
        ],
        color,
    );
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// The whole HUD for one frame.
pub fn render(batch: &mut HudBatch, atlas: &HudAtlas, registry: &FeatureRegistry, data: &HudData) {
    let screen_w = batch.width;
    let screen_h = batch.height;
    let view_proj =
        glam::Mat4::from_cols_array(&data.cam_proj) * glam::Mat4::from_cols_array(&data.cam_view);
    let project = |p: [f64; 3]| project_point(p, data.cam_pos, &view_proj, screen_w, screen_h);

    draw_esp(batch, atlas, registry, data, &view_proj);
    draw_storage(batch, atlas, registry, data, &view_proj);
    draw_waypoints(batch, atlas, registry, data, &project);
    draw_trails(batch, atlas, registry, data, &project);
    draw_jump_rings(batch, atlas, registry, data, &project);
    draw_block_highlight(batch, atlas, registry, data, &view_proj);
    draw_tracers(batch, atlas, registry, data, &view_proj);

    draw_watermark(batch, atlas, registry, data);
    draw_coords_hud(batch, atlas, registry, data);
    draw_module_list(batch, atlas, registry);
    draw_totem_counter(batch, atlas, registry, data);
    draw_target_hud(batch, atlas, registry, data);
    draw_crosshair(batch, atlas, registry, screen_w, screen_h, data);
    draw_fov_circle(batch, atlas, registry, data.cam_fov, screen_w, screen_h);
    draw_crit_indicator(batch, atlas, registry, screen_w, screen_h, data);
}

fn draw_esp(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
    view_proj: &glam::Mat4,
) {
    let box_on = registry.is_enabled("esp_box");
    let names_on = registry.is_enabled("esp_names");
    let health_on = registry.is_enabled("esp_health");
    if !box_on && !names_on && !health_on {
        return;
    }
    let esp = registry.get_feature("esp_box");
    let style = esp.map_or(0, |f| f.get_enum_setting("box_style"));
    let thickness = esp.map_or(1.5, |f| f.get_float_setting("thickness"));
    let max_range = esp.map_or(128.0, |f| f.get_float_setting("max_range"));
    let show_players = esp.is_none_or(|f| f.get_bool_setting("show_players"));
    let show_monsters = esp.is_none_or(|f| f.get_bool_setting("show_monsters"));
    let show_passives = esp.is_some_and(|f| f.get_bool_setting("show_passives"));
    let health_tint = esp.is_some_and(|f| f.get_bool_setting("health_color"));
    let c_player = registry.color_of("esp_box", "color_player", [90, 160, 255, 230]);
    let c_monster = registry.color_of("esp_box", "color_monster", [255, 70, 70, 230]);
    let c_passive = registry.color_of("esp_box", "color_passive", [80, 235, 120, 230]);
    let name_max = registry
        .get_feature("esp_names")
        .map_or(64.0, |f| f.get_float_setting("max_dist"));
    let show_distance = registry
        .get_feature("esp_names")
        .is_none_or(|f| f.get_bool_setting("show_distance"));
    let show_health = registry
        .get_feature("esp_names")
        .is_some_and(|f| f.get_bool_setting("show_health"));

    for &(_, _, kind_id, custom_name, pos, bounds, health, max_health) in data.mobs {
        if health <= 0.0 {
            continue;
        }
        let dx = (pos[0] - data.cam_pos[0]) as f32;
        let dy = (pos[1] - data.cam_pos[1]) as f32;
        let dz = (pos[2] - data.cam_pos[2]) as f32;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist > max_range {
            continue;
        }

        let is_player = kind_id.ends_with("player");
        let is_passive = super::combat_is_passive(kind_id);
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

        let base = if is_player {
            c_player
        } else if is_hostile {
            c_monster
        } else {
            c_passive
        };
        let color = if health_tint {
            theme::mix(
                rgba(base[0], base[1], base[2], base[3]),
                health_color(health, max_health),
                0.7,
            )
        } else {
            rgba(base[0], base[1], base[2], base[3])
        };

        let Some(([x0, y0], [x1, y1])) =
            project_aabb(bounds, data.cam_pos, view_proj, batch.width, batch.height)
        else {
            continue;
        };
        let bw = (x1 - x0).max(4.0);
        let bh = (y1 - y0).max(4.0);

        if box_on {
            if style == 1 {
                draw_corner_box(batch, atlas, x0, y0, bw, bh, thickness, color);
            } else {
                draw_rect_outline(batch, atlas, x0, y0, bw, bh, thickness, color);
            }
        }
        if health_on && registry
            .get_feature("esp_health")
            .is_none_or(|f| f.get_bool_setting("health_bar"))
        {
            draw_health_bar(batch, atlas, x0, y0, bh, health, max_health);
        }
        if names_on && dist <= name_max {
            let name = custom_name.unwrap_or_else(|| kind_id.trim_start_matches("minecraft:"));
            let mut label = truncate(name, 18);
            if show_health {
                label.push_str(&format!(" {:.0}", health));
            }
            if show_distance {
                label.push_str(&format!(" [{dist:.0}m]"));
            }
            let tw = HudBatch::text_width(atlas, &label);
            let lx = x0 + (bw - tw) * 0.5;
            let ly = (y0 - 11.0).max(1.0);
            theme::rounded_rect(batch, atlas, lx - 3.0, ly - 1.0, tw + 6.0, 11.0, 3.0, rgba(0, 0, 0, 160));
            batch.text(atlas, &label, lx, ly, rgba(255, 255, 255, 255), true);
        }
    }
}

fn draw_storage(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
    view_proj: &glam::Mat4,
) {
    if !registry.is_enabled("storage_esp") || data.storage.is_empty() {
        return;
    }
    let color = registry.color_of("storage_esp", "color", [255, 200, 60, 220]);
    let max = registry
        .get_feature("storage_esp")
        .map_or(96.0, |f| f.get_float_setting("max_dist"));
    for &(kind, pos) in data.storage {
        let dx = (pos[0] - data.cam_pos[0]) as f32;
        let dy = (pos[1] - data.cam_pos[1]) as f32;
        let dz = (pos[2] - data.cam_pos[2]) as f32;
        if (dx * dx + dy * dy + dz * dz).sqrt() > max {
            continue;
        }
        let bounds = Aabb::block(horizon_core::position::BlockPos::new(
            pos[0].floor() as i32,
            pos[1].floor() as i32,
            pos[2].floor() as i32,
        ));
        if let Some(([x0, y0], [x1, y1])) =
            project_aabb(bounds, data.cam_pos, view_proj, batch.width, batch.height)
        {
            let c = if kind == 2 {
                rgba(255, 120, 60, 220)
            } else {
                rgba(color[0], color[1], color[2], color[3])
            };
            draw_rect_outline(batch, atlas, x0, y0, (x1 - x0).max(3.0), (y1 - y0).max(3.0), 1.2, c);
        }
    }
}

fn draw_waypoints(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
    project: &impl Fn([f64; 3]) -> Option<[f32; 2]>,
) {
    if !registry.is_enabled("waypoints") || data.waypoints.is_empty() {
        return;
    }
    let feat = registry.get_feature("waypoints");
    let max = feat.map_or(512.0, |f| f.get_float_setting("max_dist"));
    let beam = feat.is_none_or(|f| f.get_bool_setting("beam"));
    let label = feat.is_none_or(|f| f.get_bool_setting("label"));

    for wp in data.waypoints {
        if !wp.dimension.is_empty() && wp.dimension != data.dimension {
            continue;
        }
        let dx = (wp.x - data.cam_pos[0]) as f32;
        let dy = (wp.y - data.cam_pos[1]) as f32;
        let dz = (wp.z - data.cam_pos[2]) as f32;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist > max {
            continue;
        }
        let Some([sx, sy]) = project([wp.x, wp.y + 1.0, wp.z]) else {
            continue;
        };
        let c = rgba(wp.color[0], wp.color[1], wp.color[2], wp.color[3]);
        if beam {
            if let Some([tx, ty]) = project([wp.x, wp.y + 40.0, wp.z]) {
                line(batch, atlas, [sx, sy], [tx, ty], 1.5, theme::with_alpha(c, 90));
            }
        }
        // Diamond marker.
        batch.fill_quad(
            atlas,
            [
                [sx, sy - 4.0],
                [sx + 4.0, sy],
                [sx, sy + 4.0],
                [sx - 4.0, sy],
            ],
            c,
        );
        if label {
            let text = format!("{} {:.0}m", truncate(&wp.name, 14), dist);
            let tw = HudBatch::text_width(atlas, &text);
            theme::rounded_rect(batch, atlas, sx + 6.0, sy - 5.0, tw + 6.0, 11.0, 3.0, rgba(0, 0, 0, 150));
            batch.text(atlas, &text, sx + 9.0, sy - 4.0, rgba(255, 255, 255, 255), false);
        }
    }
}

fn draw_trails(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
    project: &impl Fn([f64; 3]) -> Option<[f32; 2]>,
) {
    if !registry.is_enabled("trails") || data.trails.len() < 2 {
        return;
    }
    let feat = registry.get_feature("trails");
    let color = feat.map_or([124, 92, 255, 200], |f| f.get_color_setting("color"));
    let width = feat.map_or(2.0, |f| f.get_float_setting("width"));
    let points: Vec<[f32; 2]> = data
        .trails
        .iter()
        .filter_map(|p| project([p[0], p[1] + 1.0, p[2]]))
        .collect();
    for i in 1..points.len() {
        let t = i as f32 / points.len() as f32;
        let a = (color[3] as f32 * t) as u8;
        line(
            batch,
            atlas,
            points[i - 1],
            points[i],
            width * (0.4 + t),
            rgba(color[0], color[1], color[2], a),
        );
    }
}

fn draw_jump_rings(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
    project: &impl Fn([f64; 3]) -> Option<[f32; 2]>,
) {
    if !registry.is_enabled("jump_circles") || data.jump_rings.is_empty() {
        return;
    }
    let feat = registry.get_feature("jump_circles");
    let color = feat.map_or([124, 92, 255, 220], |f| f.get_color_setting("color"));
    let size = feat.map_or(1.0, |f| f.get_float_setting("size"));
    let now = Instant::now();
    for (pos, at) in data.jump_rings {
        let age = now.duration_since(*at).as_secs_f32();
        let t = (1.0 - age / 1.4).clamp(0.0, 1.0);
        if t <= 0.0 {
            continue;
        }
        let radius = size * (1.0 + (1.0 - t) * 1.5);
        let alpha = (color[3] as f32 * t) as u8;
        let c = rgba(color[0], color[1], color[2], alpha);
        let mut prev: Option<[f32; 2]> = None;
        let steps = 18;
        for i in 0..=steps {
            let a = std::f32::consts::TAU * i as f32 / steps as f32;
            let p = project([
                pos[0] + (a.cos() * radius) as f64,
                pos[1] + 0.05,
                pos[2] + (a.sin() * radius) as f64,
            ]);
            if let (Some(prev), Some(p)) = (prev, p) {
                line(batch, atlas, prev, p, 2.0, c);
            }
            prev = p;
        }
    }
}

fn draw_block_highlight(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
    view_proj: &glam::Mat4,
) {
    if !registry.is_enabled("block_highlight") {
        return;
    }
    let Some((_, bounds)) = data.target_block else {
        return;
    };
    let feat = registry.get_feature("block_highlight");
    let color = feat.map_or([124, 92, 255, 230], |f| f.get_color_setting("color"));
    let thickness = feat.map_or(1.5, |f| f.get_float_setting("thickness"));
    let fill_alpha = feat.map_or(40, |f| f.get_int_setting("fill_alpha"));
    let Some(([x0, y0], [x1, y1])) =
        project_aabb(bounds, data.cam_pos, view_proj, batch.width, batch.height)
    else {
        return;
    };
    let w = (x1 - x0).max(2.0);
    let h = (y1 - y0).max(2.0);
    if fill_alpha > 0 {
        batch.fill(
            atlas,
            x0,
            y0,
            w,
            h,
            rgba(color[0], color[1], color[2], fill_alpha as u8),
        );
    }
    draw_rect_outline(
        batch,
        atlas,
        x0,
        y0,
        w,
        h,
        thickness,
        rgba(color[0], color[1], color[2], color[3]),
    );
}

fn draw_tracers(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
    view_proj: &glam::Mat4,
) {
    if !registry.is_enabled("tracers") {
        return;
    }
    let feat = registry.get_feature("tracers");
    let color = feat.map_or([124, 92, 255, 180], |f| f.get_color_setting("color"));
    let max = feat.map_or(128.0, |f| f.get_float_setting("max_range"));
    let origin = feat.map_or(0, |f| f.get_enum_setting("origin"));
    let start = if origin == 1 {
        [batch.width * 0.5, batch.height * 0.5]
    } else {
        [batch.width * 0.5, batch.height]
    };
    let c = rgba(color[0], color[1], color[2], color[3]);
    for &(_, _, kind_id, _, pos, bounds, health, _) in data.mobs {
        if health <= 0.0 {
            continue;
        }
        let dx = (pos[0] - data.cam_pos[0]) as f32;
        let dy = (pos[1] - data.cam_pos[1]) as f32;
        let dz = (pos[2] - data.cam_pos[2]) as f32;
        if (dx * dx + dy * dy + dz * dz).sqrt() > max {
            continue;
        }
        if let Some(([x0, y0], [x1, y1])) =
            project_aabb(bounds, data.cam_pos, view_proj, batch.width, batch.height)
        {
            let _ = kind_id;
            line(batch, atlas, start, [x0 + (x1 - x0) * 0.5, y0 + (y1 - y0) * 0.5], 1.2, c);
        }
    }
}

fn draw_watermark(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
) {
    if !registry.is_enabled("watermark") {
        return;
    }
    let feat = registry.get_feature("watermark");
    let style = feat.map_or(0, |f| f.get_enum_setting("style"));
    let fps = feat.is_none_or(|f| f.get_bool_setting("show_fps"));
    let brand = if style == 1 { "corz" } else { "CORZ CLIENT" };
    let mut text = brand.to_string();
    if fps {
        text.push_str(&format!("  {:.0} fps", data.fps));
    }
    let tw = HudBatch::text_width(atlas, &text);
    let w = tw + 16.0;
    theme::rounded_rect(batch, atlas, 4.0, 4.0, w, 14.0, 4.0, rgba(14, 16, 24, 210));
    batch.fill(atlas, 4.0, 4.0, 2.0, 14.0, rgba(124, 92, 255, 255));
    batch.text(atlas, &text, 10.0, 7.0, rgba(240, 245, 255, 255), true);
}

fn draw_coords_hud(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
) {
    if !registry.is_enabled("coords_hud") {
        return;
    }
    let feat = registry.get_feature("coords_hud");
    let fake = registry.get_feature("fake_coords").filter(|f| f.enabled);
    let (ox, oy, oz) = fake.map_or((0.0, 0.0, 0.0), |f| {
        (
            f.get_float_setting("offset_x"),
            f.get_float_setting("offset_y"),
            f.get_float_setting("offset_z"),
        )
    });
    let x = data.player_pos[0] + ox as f64;
    let y = data.player_pos[1] + oy as f64;
    let z = data.player_pos[2] + oz as f64;
    let mut lines = vec![
        format!("XYZ: {x:.1} / {y:.1} / {z:.1}"),
        format!("Facing: {:.1}° / {:.1}°", data.player_yaw, data.player_pitch),
    ];
    if feat.is_none_or(|f| f.get_bool_setting("show_biome")) && !data.biome.is_empty() {
        lines.push(format!(
            "Biome: {}",
            data.biome.trim_start_matches("minecraft:")
        ));
    }
    if feat.is_none_or(|f| f.get_bool_setting("show_dimension")) && !data.dimension.is_empty() {
        lines.push(format!(
            "Dim: {}",
            data.dimension.trim_start_matches("minecraft:")
        ));
    }
    if feat.is_some_and(|f| f.get_bool_setting("show_fps")) {
        lines.push(format!("FPS: {:.0}", data.fps));
    }
    let max_w = lines
        .iter()
        .map(|l| HudBatch::text_width(atlas, l))
        .fold(0.0f32, f32::max);
    let h = lines.len() as f32 * 10.0 + 4.0;
    theme::rounded_rect(batch, atlas, 4.0, 22.0, max_w + 10.0, h, 4.0, rgba(0, 0, 0, 120));
    let mut y = 25.0;
    for l in lines {
        batch.text(atlas, &l, 8.0, y, rgba(230, 235, 245, 255), true);
        y += 10.0;
    }
}

fn draw_module_list(batch: &mut HudBatch, atlas: &HudAtlas, registry: &FeatureRegistry) {
    if !registry.is_enabled("module_list") {
        return;
    }
    let feat = registry.get_feature("module_list");
    let x = feat.map_or(6.0, |f| f.get_float_setting("hud_x"));
    let mut y = feat.map_or(120.0, |f| f.get_float_setting("hud_y"));
    let show_binds = feat.is_none_or(|f| f.get_bool_setting("show_binds"));
    let by_category = feat.is_some_and(|f| f.get_enum_setting("sort") == 1);

    let mut items: Vec<&super::feature::Feature> =
        registry.features.iter().filter(|f| f.enabled).collect();
    if by_category {
        items.sort_by_key(|f| (f.category.index(), f.name.clone()));
    } else {
        items.sort_by(|a, b| a.name.cmp(&b.name));
    }
    items.truncate(18);

    for f in items {
        let mut label = f.name.clone();
        if show_binds && f.bind != "None" {
            label.push_str(&format!(" [{}]", f.bind));
        }
        let tw = HudBatch::text_width(atlas, &label);
        let w = tw + 12.0;
        theme::rounded_rect(batch, atlas, x, y, w, 12.0, 3.0, rgba(14, 16, 24, 190));
        batch.fill(atlas, x, y, 2.0, 12.0, rgba(124, 92, 255, 255));
        batch.text(atlas, &label, x + 7.0, y + 3.0, rgba(238, 242, 255, 255), true);
        y += 13.0;
    }
}

fn draw_totem_counter(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
) {
    if !registry.is_enabled("totem_counter") {
        return;
    }
    let feat = registry.get_feature("totem_counter");
    let x = feat.map_or(6.0, |f| f.get_float_setting("hud_x"));
    let y = feat.map_or(200.0, |f| f.get_float_setting("hud_y"));
    let include_off = feat.is_none_or(|f| f.get_bool_setting("show_offhand"));
    let n = data.totems + usize::from(include_off && data.offhand_totem);
    let label = format!("TOTEMS {n}");
    let tw = HudBatch::text_width(atlas, &label);
    theme::rounded_rect(batch, atlas, x, y, tw + 16.0, 16.0, 4.0, rgba(14, 16, 24, 200));
    theme::rounded_rect(batch, atlas, x + 4.0, y + 4.0, 8.0, 8.0, 4.0, rgba(255, 206, 84, 255));
    batch.text(atlas, &label, x + 15.0, y + 5.0, rgba(255, 236, 180, 255), true);
}

fn draw_target_hud(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    data: &HudData,
) {
    if !registry.is_enabled("target_hud") {
        return;
    }
    let feat = registry.get_feature("target_hud");
    let x = feat.map_or(10.0, |f| f.get_float_setting("hud_x"));
    let y = feat.map_or(30.0, |f| f.get_float_setting("hud_y"));
    // The combat target is published by the game through `current_target`;
    // the HUD draws the closest player as a fallback while it is empty.
    let target = data
        .mobs
        .iter()
        .filter(|m| m.6 > 0.0)
        .min_by(|a, b| {
            let da = (a.4[0] - data.player_pos[0]).powi(2)
                + (a.4[1] - data.player_pos[1]).powi(2)
                + (a.4[2] - data.player_pos[2]).powi(2);
            let db = (b.4[0] - data.player_pos[0]).powi(2)
                + (b.4[1] - data.player_pos[1]).powi(2)
                + (b.4[2] - data.player_pos[2]).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
    let Some(target) = target else {
        return;
    };
    let (_, _, kind, custom, pos, _, health, max_health) = *target;
    let dist = ((pos[0] - data.player_pos[0]).powi(2)
        + (pos[1] - data.player_pos[1]).powi(2)
        + (pos[2] - data.player_pos[2]).powi(2))
    .sqrt();
    let name = custom.unwrap_or_else(|| kind.trim_start_matches("minecraft:"));

    let w = 150.0;
    let h = 44.0;
    theme::shadow(batch, atlas, x, y, w, h, 6.0, rgba(0, 0, 0, 140));
    theme::rounded_rect(batch, atlas, x, y, w, h, 6.0, rgba(15, 17, 25, 226));
    batch.fill(atlas, x, y, w, 2.0, rgba(124, 92, 255, 255));
    batch.text(atlas, &truncate(name, 18), x + 8.0, y + 7.0, rgba(255, 255, 255, 255), true);
    if feat.is_none_or(|f| f.get_bool_setting("show_distance")) {
        let d = format!("{dist:.1}m");
        let dw = HudBatch::text_width(atlas, &d);
        theme::rounded_rect(batch, atlas, x + w - dw - 12.0, y + 5.0, dw + 6.0, 10.0, 3.0, rgba(38, 42, 58, 220));
        batch.text(atlas, &d, x + w - dw - 9.0, y + 6.0, rgba(200, 214, 255, 255), false);
    }
    let pct = (health / max_health.max(1.0)).clamp(0.0, 1.0);
    if feat.is_none_or(|f| f.get_bool_setting("show_hp_bar")) {
        let bw = w - 16.0;
        batch.fill(atlas, x + 8.0, y + 26.0, bw, 5.0, rgba(28, 31, 44, 255));
        batch.fill(atlas, x + 8.0, y + 26.0, bw * pct, 5.0, health_color(health, max_health));
    }
    if feat.is_none_or(|f| f.get_bool_setting("show_armor")) {
        batch.text(
            atlas,
            &format!("HP {:.0}/{:.0}", health, max_health),
            x + 8.0,
            y + 34.0,
            rgba(176, 186, 208, 255),
            false,
        );
    }
}

fn draw_crosshair(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    screen_w: f32,
    screen_h: f32,
    data: &HudData,
) {
    if !registry.is_enabled("custom_crosshair") {
        return;
    }
    let feat = registry.get_feature("custom_crosshair");
    let style = feat.map_or(0, |f| f.get_enum_setting("style"));
    let color = feat.map_or([255, 255, 255, 230], |f| f.get_color_setting("color"));
    let size = feat.map_or(6, |f| f.get_int_setting("size")).clamp(2, 20) as f32;
    let gap = feat.map_or(3, |f| f.get_int_setting("gap")).clamp(0, 10) as f32;
    let dynamic = feat.is_none_or(|f| f.get_bool_setting("dynamic"));
    let _ = data;
    let c = rgba(color[0], color[1], color[2], color[3]);
    let cx = screen_w * 0.5;
    let cy = screen_h * 0.5;
    match style {
        1 => batch.fill(atlas, cx - 1.5, cy - 1.5, 3.0, 3.0, c),
        2 => {
            for i in 0..24 {
                let a = std::f32::consts::TAU * i as f32 / 24.0;
                batch.fill(
                    atlas,
                    cx + a.cos() * gap - 0.7,
                    cy + a.sin() * gap - 0.7,
                    1.4,
                    1.4,
                    c,
                );
            }
        }
        _ => {
            let g = if dynamic { gap } else { gap + 1.0 };
            batch.fill(atlas, cx - size - g, cy - 0.5, size, 1.0, c);
            batch.fill(atlas, cx + g, cy - 0.5, size, 1.0, c);
            batch.fill(atlas, cx - 0.5, cy - size - g, 1.0, size, c);
            batch.fill(atlas, cx - 0.5, cy + g, 1.0, size, c);
        }
    }
}

fn fov_radius_pixels(fov_deg: f32, camera_fov_deg: f32, screen_h: f32) -> f32 {
    let half = (fov_deg.clamp(1.0, 178.0) * 0.5).to_radians();
    let camera_half = (camera_fov_deg.clamp(1.0, 178.0) * 0.5).to_radians();
    screen_h * 0.5 * half.tan() / camera_half.tan().max(0.001)
}

fn draw_fov_circle(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    camera_fov_deg: f32,
    screen_w: f32,
    screen_h: f32,
) {
    let Some(feat) = registry.get_feature("fov_circle").filter(|f| f.enabled) else {
        return;
    };
    let thickness = feat.get_float_setting("thickness").clamp(1.0, 6.0);
    let radius = if feat.get_bool_setting("sync") {
        let combat_fov = ["silent_aura", "aim_assist"]
            .into_iter()
            .filter(|id| registry.is_enabled(id))
            .filter_map(|id| registry.get_feature(id))
            .map(|f| f.get_float_setting("fov"))
            .find(|fov| *fov < 179.0)
            .unwrap_or_else(|| feat.get_float_setting("fov"));
        fov_radius_pixels(combat_fov, camera_fov_deg, screen_h)
    } else {
        feat.get_float_setting("radius")
    };
    let radius = radius.clamp(4.0, (screen_w.max(screen_h) * 2.0).max(1.0));
    let color = feat.get_color_setting("color");
    let cx = screen_w * 0.5;
    let cy = screen_h * 0.5;
    let segments = (radius * 0.7).clamp(48.0, 160.0) as usize;
    let step = std::f32::consts::TAU / segments as f32;
    for (ring_radius, ring_thickness, ring_color) in [
        (radius, thickness + 2.0, rgba(0, 0, 0, 120)),
        (radius, thickness, rgba(color[0], color[1], color[2], color[3])),
    ] {
        let inner = (ring_radius - ring_thickness).max(1.0);
        for i in 0..segments {
            let a0 = step * i as f32;
            let a1 = step * (i + 1) as f32;
            let (s0, c0) = a0.sin_cos();
            let (s1, c1) = a1.sin_cos();
            batch.fill_quad(
                atlas,
                [
                    [cx + c0 * inner, cy + s0 * inner],
                    [cx + c1 * inner, cy + s1 * inner],
                    [cx + c1 * ring_radius, cy + s1 * ring_radius],
                    [cx + c0 * ring_radius, cy + s0 * ring_radius],
                ],
                ring_color,
            );
        }
    }
}

fn draw_crit_indicator(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    registry: &FeatureRegistry,
    screen_w: f32,
    screen_h: f32,
    data: &HudData,
) {
    if !registry.is_enabled("crit_assist") {
        return;
    }
    let Some(feat) = registry.get_feature("crit_assist") else {
        return;
    };
    if !feat.get_bool_setting("hud_indicator") {
        return;
    }
    // The game publishes the charge through the registry-independent combat
    // state; this badge only needs the falling flag.
    let ready = data.falling && data.attack_scale >= 0.9;
    let (text, color) = if ready {
        ("CRIT READY", rgba(255, 220, 50, 255))
    } else if !data.falling {
        ("JUMP TO CRIT", rgba(180, 190, 205, 190))
    } else {
        ("WAIT FOR FALL", rgba(255, 160, 60, 220))
    };
    let tw = HudBatch::text_width(atlas, text);
    let bw = tw + 14.0;
    let bx = (screen_w - bw) * 0.5;
    let by = screen_h * 0.5 + 22.0;
    theme::rounded_rect(batch, atlas, bx, by, bw, 13.0, 4.0, rgba(22, 24, 34, 210));
    batch.text(atlas, text, bx + 7.0, by + 3.0, color, true);
}
