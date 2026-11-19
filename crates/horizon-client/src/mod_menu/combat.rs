//! Modular combat testing and target selection framework.

use super::feature::FeatureRegistry;
use std::time::{Duration, Instant};

/// Information on an entity detected by the combat system.
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

/// Combat controller managing target evaluation and rate-limited simulated attacks.
pub struct CombatSystem {
    pub last_attack_time: Instant,
    pub silent_aura_last_attack: Instant,
    pub trigger_bot_last_attack: Instant,
    pub current_target: Option<TargetCandidate>,
    pub is_falling: bool,
    pub is_on_ground: bool,
    pub attack_cooldown_scale: f32,
}

impl Default for CombatSystem {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            last_attack_time: now,
            silent_aura_last_attack: now,
            trigger_bot_last_attack: now,
            current_target: None,
            is_falling: false,
            is_on_ground: true,
            attack_cooldown_scale: 1.0,
        }
    }
}

impl CombatSystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates target candidates for a specific combat feature.
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
        let target_monsters = feat.get_bool_setting("target_monsters");
        let target_players = feat.get_bool_setting("target_players");
        let target_passives = feat.get_bool_setting("target_passives");
        let priority_mode = feat.get_enum_setting("priority");

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

            if is_player && !target_players {
                continue;
            }
            if is_hostile && !target_monsters {
                continue;
            }
            if is_passive && !target_passives {
                continue;
            }

            // Calculate angle to player's view vector
            let angle = calculate_angle_diff(player_yaw, player_pitch, dx, dy, dz);

            // Field of view: only entities inside the aim cone (360 = all around).
            if fov > 0.0 && angle > (fov.clamp(1.0, 360.0) * 0.5) {
                continue;
            }

            let display_name = custom_name
                .unwrap_or_else(|| kind_id.trim_start_matches("minecraft:"))
                .to_string();

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

        // Sort by selected priority
        match priority_mode {
            0 => {
                // Closest
                candidates.sort_by(|a, b| {
                    a.distance
                        .partial_cmp(&b.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            1 => {
                // Lowest HP
                candidates.sort_by(|a, b| {
                    a.health
                        .partial_cmp(&b.health)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            2 => {
                // Crosshair angle
                candidates.sort_by(|a, b| {
                    a.angle
                        .partial_cmp(&b.angle)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            _ => {}
        }

        let best = candidates.into_iter().next();
        self.current_target = best.clone();
        best
    }

    /// Evaluates target candidates from the game's mob list based on configured settings (combat_test).
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

    /// Checks if a rate-limited attack should be performed now for a given feature.
    ///
    /// The interval is the slower of the feature's `aps` (attacks per second)
    /// and its `delay` (milliseconds), so triggerbot (delay only) and the auras
    /// (aps, optionally with an extra delay) share one timer.
    pub fn should_attack_feature(
        &mut self,
        feature_name: &str,
        registry: &FeatureRegistry,
    ) -> bool {
        let feat = match registry.get_feature(feature_name) {
            Some(f) if f.enabled => f,
            _ => return false,
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

    /// Checks if a rate-limited attack should be performed now based on APS setting (combat_test).
    pub fn should_attack(&mut self, registry: &FeatureRegistry) -> bool {
        self.should_attack_feature("combat_test", registry)
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

/// Calculates angular difference between player view and target direction in degrees.
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

/// Wraps an angle in degrees into `-180..180` (`Mth.wrapDegrees`).
pub fn wrap_degrees(deg: f32) -> f32 {
    let mut d = (deg + 180.0) % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    d - 180.0
}

/// The view rotation (yaw, pitch) that looks from `eye` at `target`.
///
/// Same convention as `Game::view_vector`: `-sin(yaw)cos(pitch)`,
/// `-sin(pitch)`, `cos(yaw)cos(pitch)`.
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
