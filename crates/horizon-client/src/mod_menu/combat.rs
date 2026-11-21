//! Combat targeting, rate limits and the small pieces of state the Corz
//! modules share (W-tap window, hit-select window, trails, jump rings).

use super::feature::FeatureRegistry;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct TargetCandidate {
    pub index: usize,
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub position: [f64; 3],
    pub distance: f32,
    pub health: f32,
    pub max_health: f32,
    pub angle: f32,
}

pub struct CombatSystem {
    pub last_attack_time: Instant,
    pub silent_aura_last_attack: Instant,
    pub trigger_bot_last_attack: Instant,
    pub auto_clicker_last: Instant,
    pub current_target: Option<TargetCandidate>,
    pub is_falling: bool,
    pub is_on_ground: bool,
    pub attack_cooldown_scale: f32,
    /// When the auto-critical jump was tapped. Attacks wait for the fall
    /// while it is recent, so the weapon charge survives the jump.
    pub auto_crit_jump_at: Option<Instant>,
    /// W-tap: sprint is released until this instant.
    pub wtap_until: Option<Instant>,
    /// Hit select: attacks are held until this instant.
    pub hit_select_until: Option<Instant>,
    /// Last health seen, for Auto Jump Reset.
    pub last_health: f32,
    pub pot_at: Instant,
    pub xp_at: Instant,
    pub pearl_at: Instant,
    /// (slot to restore, when) after a Key Pearl swap.
    pub pearl_restore: Option<(usize, Instant)>,
    pub zoom: bool,
    /// 0..1 eased zoom factor.
    pub zoom_progress: f32,
    /// Recent player positions for the trail.
    pub trails: Vec<[f64; 3]>,
    pub trail_at: Instant,
    /// (position, when) of the last jump rings.
    pub jump_rings: Vec<([f64; 3], Instant)>,
    /// Storage positions collected from the world's block entities.
    pub storage: Vec<(u8, [f64; 3])>,
    pub storage_at: Instant,
    pub last_attack_at: Option<Instant>,
    pub last_ground: bool,
    /// Hotbar slot to return to when Auto Tool finishes a block.
    pub prev_tool_slot: Option<usize>,
    /// Last Inventory Cleaner sweep.
    pub cleaner_at: Instant,
    /// Last Trident Assist throw or riptide.
    pub trident_at: Instant,
}

impl Default for CombatSystem {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            last_attack_time: now,
            silent_aura_last_attack: now,
            trigger_bot_last_attack: now,
            auto_clicker_last: now,
            current_target: None,
            is_falling: false,
            is_on_ground: true,
            attack_cooldown_scale: 1.0,
            auto_crit_jump_at: None,
            wtap_until: None,
            hit_select_until: None,
            last_health: 20.0,
            pot_at: now,
            xp_at: now,
            pearl_at: now,
            pearl_restore: None,
            zoom: false,
            zoom_progress: 0.0,
            trails: Vec::new(),
            trail_at: now,
            jump_rings: Vec::new(),
            storage: Vec::new(),
            storage_at: now,
            last_attack_at: None,
            last_ground: true,
            prev_tool_slot: None,
            cleaner_at: now,
            trident_at: now,
        }
    }
}

impl CombatSystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates target candidates for a combat feature. `friends` are skipped
    /// by every module, as in CorzClient.
    #[allow(clippy::too_many_arguments)]
    pub fn find_target_for<'a, I>(
        &mut self,
        feature_name: &str,
        registry: &FeatureRegistry,
        player_pos: [f64; 3],
        player_yaw: f32,
        player_pitch: f32,
        mobs_iter: I,
    ) -> Option<TargetCandidate>
    where
        I: Iterator<Item = (usize, u32, &'a str, Option<&'a str>, [f64; 3], f32, f32)>,
    {
        let feat = registry.get_feature(feature_name)?;
        if !feat.enabled {
            return None;
        }

        let max_range = feat.get_float_setting("range").clamp(1.0, 10.0);
        let fov = feat.get_float_setting("fov");
        let priority_mode = feat.get_enum_setting("priority");

        // New-style `targets` enum, or the legacy per-kind toggles.
        let targets = feat.setting("targets").map(|s| s.as_enum());
        let (players, monsters, passives) = match targets {
            Some(0) => (true, false, false),
            Some(1) => (true, true, false),
            Some(2) => (false, true, false),
            _ => (
                feat.get_bool_setting("target_players"),
                feat.get_bool_setting("target_monsters"),
                feat.get_bool_setting("target_passives"),
            ),
        };

        let mut candidates = Vec::new();

        for (index, id, kind_id, custom_name, pos, health, max_health) in mobs_iter {
            if health <= 0.0 {
                continue;
            }

            let dx = (pos[0] - player_pos[0]) as f32;
            let dy = (pos[1] - player_pos[1]) as f32;
            let dz = (pos[2] - player_pos[2]) as f32;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            if dist > max_range {
                continue;
            }

            let is_player = kind_id.ends_with("player");
            let is_passive = is_passive_mob(kind_id);
            let is_hostile = !is_player && !is_passive;

            if is_player && !players {
                continue;
            }
            if is_hostile && !monsters {
                continue;
            }
            if is_passive && !passives {
                continue;
            }

            let display_name = custom_name
                .unwrap_or_else(|| kind_id.trim_start_matches("minecraft:"))
                .to_string();
            if is_player && registry.is_friend(&display_name) {
                continue;
            }

            let angle = calculate_angle_diff(player_yaw, player_pitch, dx, dy, dz);
            if fov > 0.0 && angle > (fov.clamp(1.0, 360.0) * 0.5) {
                continue;
            }

            candidates.push(TargetCandidate {
                index,
                id,
                name: display_name,
                kind: kind_id.to_string(),
                position: pos,
                distance: dist,
                health,
                max_health,
                angle,
            });
        }

        match priority_mode {
            1 => candidates.sort_by(|a, b| {
                a.health
                    .partial_cmp(&b.health)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            2 => candidates.sort_by(|a, b| {
                a.angle
                    .partial_cmp(&b.angle)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            _ => candidates.sort_by(|a, b| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }

        let best = candidates.into_iter().next();
        self.current_target = best.clone();
        best
    }

    pub fn find_best_target<'a, I>(
        &mut self,
        registry: &FeatureRegistry,
        player_pos: [f64; 3],
        player_yaw: f32,
        player_pitch: f32,
        mobs_iter: I,
    ) -> Option<TargetCandidate>
    where
        I: Iterator<Item = (usize, u32, &'a str, Option<&'a str>, [f64; 3], f32, f32)>,
    {
        self.find_target_for(
            "combat_test",
            registry,
            player_pos,
            player_yaw,
            player_pitch,
            mobs_iter,
        )
    }

    /// Rate limit for a feature: `aps` and `delay` (ms) combined.
    pub fn should_attack_feature(
        &mut self,
        feature_name: &str,
        registry: &FeatureRegistry,
    ) -> bool {
        let Some(feat) = registry.get_feature(feature_name).filter(|f| f.enabled) else {
            return false;
        };
        let aps = feat.get_float_setting("aps");
        let aps_secs = if aps > 0.0 {
            1.0 / aps.clamp(1.0, 20.0)
        } else {
            0.0
        };
        let delay_secs = feat.get_float_setting("delay").max(0.0) / 1000.0;
        let interval = Duration::from_secs_f32(aps_secs.max(delay_secs));
        let now = Instant::now();
        let last_time = match feature_name {
            "silent_aura" => &mut self.silent_aura_last_attack,
            "trigger_bot" => &mut self.trigger_bot_last_attack,
            _ => &mut self.last_attack_time,
        };
        if now.duration_since(*last_time) >= interval {
            *last_time = now;
            true
        } else {
            false
        }
    }

    pub fn should_attack(&mut self, registry: &FeatureRegistry) -> bool {
        self.should_attack_feature("combat_test", registry)
    }

    /// Auto Clicker's own CPS clock, with optional jitter.
    pub fn should_auto_click(&mut self, registry: &FeatureRegistry) -> bool {
        let Some(feat) = registry.get_feature("auto_clicker").filter(|f| f.enabled) else {
            return false;
        };
        let cps = feat.get_float_setting("cps").clamp(1.0, 20.0);
        let jitter = feat.get_int_setting("jitter").max(0) as f32 / 1000.0;
        let mut interval = 1.0 / cps;
        if jitter > 0.0 {
            // A cheap pseudo-random 0..1 from the clock.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos());
            let n = (nanos % 1000) as f32 / 1000.0;
            interval += (n - 0.5) * jitter;
        }
        let now = Instant::now();
        if now.duration_since(self.auto_clicker_last) >= Duration::from_secs_f32(interval.max(0.02))
        {
            self.auto_clicker_last = now;
            true
        } else {
            false
        }
    }

    /// Hit Select holds attacks while the target's hurt window is open.
    pub fn clicks_held(&self) -> bool {
        self.hit_select_until.is_some_and(|t| Instant::now() < t)
    }

    /// Records a mob attack: the W-tap sprint release and the Hit Select hold
    /// window open per their own filters (`only_players`, `only_sprint_hits`).
    pub fn note_attack(
        &mut self,
        registry: &FeatureRegistry,
        is_player: bool,
        sprint_hit: bool,
    ) {
        let now = Instant::now();
        self.last_attack_at = Some(now);
        if let Some(f) = registry.get_feature("hit_select").filter(|f| f.enabled) {
            if !f.get_bool_setting("only_players") || is_player {
                let ms = f.get_int_setting("delay").clamp(0, 300) as u64 + 380;
                self.hit_select_until = Some(now + Duration::from_millis(ms));
            }
        }
        if let Some(f) = registry.get_feature("w_tap").filter(|f| f.enabled) {
            if !f.get_bool_setting("only_sprint_hits") || sprint_hit {
                let ticks = f.get_int_setting("hold_ticks").clamp(1, 6) as u64;
                self.wtap_until = Some(now + Duration::from_millis(ticks * 50));
            }
        }
    }

    pub fn in_wtap(&self) -> bool {
        self.wtap_until.is_some_and(|t| Instant::now() < t)
    }

    /// Records a trail point at most every 60 ms.
    pub fn push_trail(&mut self, pos: [f64; 3], max: usize) {
        let now = Instant::now();
        if now.duration_since(self.trail_at) < Duration::from_millis(60) {
            return;
        }
        self.trail_at = now;
        if self
            .trails
            .last()
            .is_none_or(|p| (p[0] - pos[0]).abs() + (p[1] - pos[1]).abs() + (p[2] - pos[2]).abs() > 0.05)
        {
            self.trails.push(pos);
            let max = max.clamp(4, 64);
            if self.trails.len() > max {
                let drop = self.trails.len() - max;
                self.trails.drain(0..drop);
            }
        }
    }

    pub fn push_jump_ring(&mut self, pos: [f64; 3]) {
        let now = Instant::now();
        self.jump_rings.push((pos, now));
        self.jump_rings
            .retain(|(_, at)| now.duration_since(*at) < Duration::from_millis(1400));
    }
}

/// Helper identifying friendly/passive entities.
fn is_passive_mob(kind: &str) -> bool {
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

fn calculate_angle_diff(yaw: f32, pitch: f32, dx: f32, dy: f32, dz: f32) -> f32 {
    let horiz_dist = (dx * dx + dz * dz).sqrt().max(0.001);
    let target_yaw = (-dx).atan2(dz).to_degrees();
    let target_pitch = (-dy).atan2(horiz_dist).to_degrees();

    let diff_yaw = (yaw - target_yaw).abs().rem_euclid(360.0);
    let yaw_angle = if diff_yaw > 180.0 {
        360.0 - diff_yaw
    } else {
        diff_yaw
    };
    let pitch_angle = (pitch - target_pitch).abs();

    (yaw_angle * yaw_angle + pitch_angle * pitch_angle).sqrt()
}

pub fn wrap_degrees(deg: f32) -> f32 {
    let mut d = (deg + 180.0) % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    d - 180.0
}

/// The view rotation (yaw, pitch) that looks from `eye` at `target`.
pub fn look_at(eye: [f64; 3], target: [f64; 3]) -> (f32, f32) {
    let dx = (target[0] - eye[0]) as f32;
    let dy = (target[1] - eye[1]) as f32;
    let dz = (target[2] - eye[2]) as f32;
    let horiz = (dx * dx + dz * dz).sqrt().max(0.001);
    (
        wrap_degrees((-dx).atan2(dz).to_degrees()),
        (-dy).atan2(horiz).to_degrees().clamp(-90.0, 90.0),
    )
}
