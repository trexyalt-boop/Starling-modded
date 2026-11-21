//! The Corz modules' gameplay logic, kept out of `game.rs` so it can reach
//! `Game`'s private fields. Each module that has a hook here runs from
//! `tick_corz_features` every game tick.

use crate::game::Game;
use crate::mod_menu::UiAction;
use horizon_client::items::ItemStack;
use horizon_client_render::chunk::block_entity::BlockEntity;
use std::time::{Duration, Instant};

impl Game<'_> {
    /// Everything the Corz modules do each game tick that `tick_mod_menu`
    /// does not already handle (combat bots, aim, crits live there).
    pub(super) fn tick_corz_features(&mut self) {
        // A menu tap that needs the world.
        match self.mod_menu.take_external_action() {
            Some(UiAction::AddFriend) => self.corz_quick_friend(),
            Some(UiAction::AddWaypoint) => self.corz_add_waypoint(),
            _ => {}
        }
        // One-shot requests raised by a key.
        if std::mem::take(&mut self.mod_menu.waypoint_requested) {
            self.corz_add_waypoint();
        }
        if std::mem::take(&mut self.mod_menu.quick_friend_requested) {
            self.corz_quick_friend();
        }
        if std::mem::take(&mut self.mod_menu.pearl_requested) {
            self.corz_key_pearl();
        }

        self.tick_zoom();
        self.tick_storage_cache();
        self.tick_trails_and_rings();
        self.tick_movement();
        self.tick_auto_pot_xp();
        self.tick_auto_clicker();
        self.tick_auto_jump_reset();
        self.tick_auto_mace();
        self.tick_inventory_cleaner();
        self.tick_auto_tool_swap_back();
        self.tick_pearl_restore();
        self.tick_weather();
    }

    // ------------------------------------------------------------- zoom

    fn tick_zoom(&mut self) {
        let enabled = self.mod_menu.registry.is_enabled("zoom");
        let combat = &mut self.mod_menu.combat;
        // The key toggles; `combat.zoom` remembers the state.
        let target = if enabled && combat.zoom { 1.0 } else { 0.0 };
        let smooth = self
            .mod_menu
            .registry
            .get_feature("zoom")
            .is_none_or(|f| f.get_bool_setting("smooth"));
        if smooth {
            let cur = combat.zoom_progress;
            combat.zoom_progress = cur + (target - cur) * 0.22;
            if (target - cur).abs() < 0.01 {
                combat.zoom_progress = target;
            }
        } else {
            combat.zoom_progress = target;
        }
    }

    // ------------------------------------------------------- storage ESP

    fn tick_storage_cache(&mut self) {
        if !self.mod_menu.registry.is_enabled("storage_esp") {
            self.mod_menu.combat.storage.clear();
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.mod_menu.combat.storage_at) < Duration::from_millis(500) {
            return;
        }
        self.mod_menu.combat.storage_at = now;

        let feat = self.mod_menu.registry.get_feature("storage_esp");
        let chests = feat.is_none_or(|f| f.get_bool_setting("chests"));
        let spawners = feat.is_none_or(|f| f.get_bool_setting("spawners"));
        let other = feat.is_none_or(|f| f.get_bool_setting("other"));

        let player = self.player.as_ref();
        let (px, pz) = player.map_or((0.0, 0.0), |p| (p.position.x, p.position.z));
        let radius = 112.0;
        let mut found: Vec<(u8, [f64; 3])> = Vec::new();
        for ((x, y, z), entity) in self.world.block_entities.iter_mut() {
            let dist = ((*x as f64 - px).powi(2) + (*z as f64 - pz).powi(2)).sqrt();
            if dist > radius {
                continue;
            }
            let kind = match entity {
                BlockEntity::Chest(_)
                | BlockEntity::Beehive(_)
                | BlockEntity::Vault(_)
                | BlockEntity::Crafter(_)
                | BlockEntity::Shelf(_)
                | BlockEntity::ChiseledBookshelf(_) => {
                    if chests {
                        0
                    } else {
                        continue;
                    }
                }
                BlockEntity::Spawner(_) | BlockEntity::TrialSpawner(_) => {
                    if spawners {
                        1
                    } else {
                        continue;
                    }
                }
                BlockEntity::Furnace(_)
                | BlockEntity::Brewing(_)
                | BlockEntity::Beacon(_)
                | BlockEntity::Lectern(_)
                | BlockEntity::Jukebox(_)
                | BlockEntity::CommandBlock(_)
                | BlockEntity::Campfire(_)
                | BlockEntity::EndGateway(_) => {
                    if other {
                        2
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };
            found.push((kind, [*x as f64 + 0.5, *y as f64 + 0.5, *z as f64 + 0.5]));
        }
        found.truncate(300);
        self.mod_menu.combat.storage = found;
    }

    // ------------------------------------------------- trails / rings

    fn tick_trails_and_rings(&mut self) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let pos = [p.position.x, p.position.y, p.position.z];
        if self.mod_menu.registry.is_enabled("trails") {
            let len = self
                .mod_menu
                .registry
                .get_feature("trails")
                .map_or(24, |f| f.get_int_setting("length"));
            self.mod_menu.combat.push_trail(pos, len.max(4) as usize);
        } else if !self.mod_menu.combat.trails.is_empty() {
            self.mod_menu.combat.trails.clear();
        }

        // Jump rings: stamp one at the base when a jump leaves or lands.
        if self.mod_menu.registry.is_enabled("jump_circles") {
            let was_ground = self.mod_menu.combat.last_ground;
            let ground = p.ground.on_ground;
            if was_ground != ground && !p.fluids.was_touching_water {
                self.mod_menu
                    .combat
                    .push_jump_ring([pos[0], p.position.y - 0.1, pos[2]]);
            }
            self.mod_menu.combat.last_ground = ground;
        }
    }

    // ------------------------------------------------------- movement

    fn tick_movement(&mut self) {
        let reg = &self.mod_menu.registry;

        // W-tap: while the window is open, sprint is released.
        if reg.is_enabled("w_tap") && self.mod_menu.combat.in_wtap() {
            if let Some(p) = self.player.as_mut() {
                p.sprinting = false;
            }
        }

        // Keep Sprint: re-enable sprint after a hit. "Only Hits" narrows it to
        // a short window after an attack instead of every tick.
        if reg.is_enabled("keep_sprint") {
            let only_hits = reg
                .get_feature("keep_sprint")
                .is_some_and(|f| f.get_bool_setting("only_hits"));
            let recent_hit = self
                .mod_menu
                .combat
                .last_attack_at
                .is_some_and(|t| t.elapsed() < Duration::from_millis(700));
            if !only_hits || recent_hit {
                if let Some(p) = self.player.as_mut() {
                    if p.applied_input[1] > 0.0
                        && !p.shift_key_down
                        && !p.mobility_restricted
                        && !self.mod_menu.combat.in_wtap()
                    {
                        p.sprinting = true;
                    }
                }
            }
        }

        // Auto Walk: hold the forward key while enabled.
        let auto_walk = reg.is_enabled("auto_walk");
        if auto_walk {
            self.keys.insert(winit::keyboard::KeyCode::KeyW);
        } else {
            self.keys.remove(&winit::keyboard::KeyCode::KeyW);
        }

        // Step Assist: raise the step height with a transient attribute
        // modifier (`EdgeBackoffWorld.max_up_step` reads it). The player's
        // base is 0.6, so the modifier is the difference, on or off.
        let (step_on, step_height) = match reg.get_feature("step_helper") {
            Some(f) if f.enabled => (true, f.get_float_setting("height").clamp(0.5, 1.5)),
            _ => (false, 0.6),
        };
        if let Some(p) = self.player.as_mut() {
            let _ = p.set_modifier(
                "minecraft:step_height",
                "starling:step_helper",
                f64::from(step_height) - 0.6,
                "add_value",
                step_on,
            );
        }
    }

    /// Whether the main hand holds a melee weapon (sword, axe, mace, trident
    /// or spear), for the modules that gate on one.
    pub(super) fn mod_holding_weapon(&self) -> bool {
        self.held().is_some_and(|s| {
            let id = &*s.item;
            id.contains("sword")
                || id.contains("axe")
                || id.contains("spear")
                || id.contains("mace")
                || id.contains("trident")
        })
    }

    /// A clear line of sight between two points: blocks with collision stop it
    /// (`can_see`, as the mob AI tests it).
    pub(super) fn mod_line_of_sight(&self, from: [f64; 3], to: [f64; 3]) -> bool {
        let steps = 24;
        (1..steps).all(|i| {
            let f = f64::from(i) / f64::from(steps);
            let p = [
                from[0] + (to[0] - from[0]) * f,
                from[1] + (to[1] - from[1]) * f,
                from[2] + (to[2] - from[2]) * f,
            ];
            !self
                .res
                .facts()
                .get(self.world.block(
                    p[0].floor() as i32,
                    p[1].floor() as i32,
                    p[2].floor() as i32,
                ))
                .full_collision
        })
    }

    // ------------------------------------------------ auto pot / xp

    fn tick_auto_pot_xp(&mut self) {
        let now = Instant::now();
        let health = self.player.as_ref().map_or(20.0, |p| p.health.health());
        let reg = &self.mod_menu.registry;
        let pot = reg.is_enabled("auto_pot");
        let pot_threshold = reg
            .get_feature("auto_pot")
            .map_or(10.0, |f| f.get_float_setting("health"));
        let pot_delay = reg
            .get_feature("auto_pot")
            .map_or(500, |f| f.get_int_setting("delay")) as u64;
        let potion = reg
            .get_feature("auto_pot")
            .map_or(0, |f| f.get_enum_setting("potion"));
        let refill = reg
            .get_feature("auto_pot")
            .is_some_and(|f| f.get_bool_setting("refill"));
        let xp = reg.is_enabled("auto_xp");
        let xp_delay = reg
            .get_feature("auto_xp")
            .map_or(250, |f| f.get_int_setting("delay")) as u64;
        let xp_threshold = reg
            .get_feature("auto_xp")
            .map_or(60, |f| f.get_int_setting("durability"));
        let want_pot = if potion == 1 {
            "regeneration"
        } else {
            "healing"
        };

        // Auto Pot: splash the chosen potion when low.
        if pot
            && health < pot_threshold
            && now.duration_since(self.mod_menu.combat.pot_at) >= Duration::from_millis(pot_delay)
        {
            if let Some(slot) = self.find_hotbar_potion(want_pot) {
                self.mod_menu.combat.pot_at = now;
                self.selected = slot;
                let _ = self.use_item();
            } else if refill && self.refill_potion(want_pot) {
                // The hotbar fills on the next tick; delay the next attempt.
                self.mod_menu.combat.pot_at = now;
            }
        }

        // Auto XP: throw experience bottles when a Mending item is worn down.
        if xp
            && self.worn_item_low(xp_threshold)
            && now.duration_since(self.mod_menu.combat.xp_at) >= Duration::from_millis(xp_delay)
        {
            if let Some(slot) = self.find_hotbar("minecraft:experience_bottle") {
                self.mod_menu.combat.xp_at = now;
                self.selected = slot;
                let _ = self.use_item();
            }
        }
    }

    /// A Mending item worn past `threshold` percent of its durability, in the
    /// pack, the off hand or the armor.
    fn worn_item_low(&self, threshold: i32) -> bool {
        let worn = |s: &ItemStack| {
            s.enchantment_level("minecraft:mending") > 0
                && s.damage > 0
                && self.items().get(&s.item).is_some_and(|i| {
                    i.max_damage > 0
                        && (s.damage as f32 / i.max_damage as f32) * 100.0
                            >= (100 - threshold.clamp(5, 100)) as f32
                })
        };
        self.inventory.iter().flatten().any(&worn)
            || self.offhand.as_ref().is_some_and(&worn)
            || self.armor.iter().flatten().any(&worn)
    }

    fn find_hotbar(&self, id: &str) -> Option<usize> {
        (0..9).find(|&s| {
            self.inventory
                .get(s)
                .and_then(|st| st.as_ref())
                .is_some_and(|st| &*st.item == id)
        })
    }

    /// Whether the stack in inventory slot `slot` is a splash potion whose
    /// contents name `want` (`healing`, `regeneration`).
    fn pot_matches(&self, slot: usize, want: &str) -> bool {
        self.inventory
            .get(slot)
            .and_then(|s| s.as_ref())
            .is_some_and(|s| {
                &*s.item == "minecraft:splash_potion"
                    && s.potion().is_some_and(|p| p.contains(want))
            })
    }

    fn find_hotbar_potion(&self, want: &str) -> Option<usize> {
        (0..9).find(|&s| self.pot_matches(s, want))
    }

    /// Moves a matching potion from the pack into the hotbar with a shift
    /// click, so Auto Pot can keep throwing. Only while no other container is
    /// open; the hotbar is filled by the menu engine's own quick-move rules.
    fn refill_potion(&mut self, want: &str) -> bool {
        if self.container.is_some() || self.creative_inventory_open() {
            return false;
        }
        let Some(pack) = (9..36).find(|&s| self.pot_matches(s, want)) else {
            return false;
        };
        self.menu_click(Some(pack), 0, horizon_client::menu::Input::QuickMove)
    }

    // ----------------------------------------------------- key pearl

    fn tick_pearl_restore(&mut self) {
        let now = Instant::now();
        let combat = &mut self.mod_menu.combat;
        if let Some((slot, at)) = combat.pearl_restore {
            if now.duration_since(at) >= Duration::from_millis(300) {
                combat.pearl_restore = None;
                self.selected = slot.min(8);
            }
        }
    }

    fn corz_key_pearl(&mut self) {
        let Some(slot) = self.find_hotbar("minecraft:ender_pearl") else {
            return;
        };
        if !self.mod_menu.registry.is_enabled("key_pearl") {
            return;
        }
        let prev = self.selected;
        self.selected = slot;
        let _ = self.use_item();
        self.mod_menu.combat.pearl_restore = Some((prev, Instant::now()));
    }

    // --------------------------------------------------- auto clicker

    fn tick_auto_clicker(&mut self) {
        if !self.mod_menu.registry.is_enabled("auto_clicker") {
            return;
        }
        let feat = self.mod_menu.registry.get_feature("auto_clicker");
        let require_mouse = feat.is_none_or(|f| f.get_bool_setting("require_mouse"));
        let weapons_only = feat.is_none_or(|f| f.get_bool_setting("only_weapons"));
        if require_mouse && !self.left {
            return;
        }
        if weapons_only {
            let weapon = self.held().is_some_and(|s| {
                let id = &*s.item;
                id.contains("sword")
                    || id.contains("axe")
                    || id.contains("spear")
                    || id.contains("mace")
                    || id.contains("trident")
            });
            if !weapon {
                return;
            }
        }
        if self
            .mod_menu
            .combat
            .should_auto_click(&self.mod_menu.registry)
        {
            self.attack();
        }
    }

    // ----------------------------------------------- auto jump reset

    fn tick_auto_jump_reset(&mut self) {
        if !self.mod_menu.registry.is_enabled("auto_jump_reset") {
            self.mod_menu.combat.last_health =
                self.player.as_ref().map_or(20.0, |p| p.health.health());
            return;
        }
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let health = p.health.health();
        let combat = &mut self.mod_menu.combat;
        let dropped = combat.last_health - health;
        combat.last_health = health;
        if dropped > 0.3 && p.ground.on_ground && !p.mobility_restricted {
            let chance = self
                .mod_menu
                .registry
                .get_feature("auto_jump_reset")
                .map_or(100, |f| f.get_int_setting("chance"));
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos());
            if ((nanos % 100) as i32) < chance.clamp(0, 100) {
                self.pad.touch_tap(horizon_client::gamepad::Button::South);
            }
        }
    }

    // --------------------------------------------------- auto mace

    /// Auto Mace: when falling with a held mace, smash the target under the
    /// crosshair once the fall is long enough. "Stun Slam" strips a raised
    /// shield with an axe first.
    fn tick_auto_mace(&mut self) {
        if !self.mod_menu.registry.is_enabled("auto_mace") {
            return;
        }
        let (min_fall, only_players, stun_slam) =
            match self.mod_menu.registry.get_feature("auto_mace") {
                Some(f) => (
                    f.get_float_setting("min_fall").clamp(0.5, 5.0),
                    f.get_bool_setting("only_players"),
                    f.get_bool_setting("stun_slam"),
                ),
                None => (1.5, true, false),
            };
        let Some((index, _)) = self.mob_hit() else {
            return;
        };
        if index >= self.mobs.len() || self.mobs[index].removed || self.mobs[index].health <= 0.0 {
            return;
        }
        if only_players && self.mobs[index].kind.id != "minecraft:player" {
            return;
        }
        let falling = self.player.as_ref().is_some_and(|p| {
            !p.ground.on_ground && p.fall_distance >= f64::from(min_fall) && !p.fall_flying
        });
        if !falling {
            return;
        }
        if stun_slam && self.mobs[index].extra.blocking {
            // Swap to an axe to strip the raised shield first.
            if let Some(axe) = (0..9).find(|&s| {
                self.inventory
                    .get(s)
                    .and_then(|st| st.as_ref())
                    .is_some_and(|st| {
                        let id = &*st.item;
                        id.contains("axe") && !id.contains("pickaxe") && !id.contains("shield")
                    })
            }) {
                self.selected = axe;
                self.mod_attack_mob(index, false);
                return;
            }
        }
        if self.held().is_some_and(|s| &*s.item == "minecraft:mace") {
            self.mod_attack_mob(index, false);
        }
    }

    // ------------------------------------------- inventory cleaner

    /// Inventory Cleaner: throws small junk stacks out of the pack, keeping
    /// tools, food and building blocks while their settings are on.
    fn tick_inventory_cleaner(&mut self) {
        if !self.mod_menu.registry.is_enabled("inventory_cleaner") {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.mod_menu.combat.cleaner_at) < Duration::from_millis(600) {
            return;
        }
        self.mod_menu.combat.cleaner_at = now;
        let (keep_tools, keep_food, keep_blocks, threshold) =
            match self.mod_menu.registry.get_feature("inventory_cleaner") {
                Some(f) => (
                    f.get_bool_setting("keep_tools"),
                    f.get_bool_setting("keep_food"),
                    f.get_bool_setting("keep_blocks"),
                    f.get_int_setting("threshold").clamp(1, 8) as u32,
                ),
                None => (true, true, true, 2),
            };
        let Some(slot) = (9..36).find(|&slot| {
            let Some(stack) = self.inventory.get(slot).and_then(|s| s.as_ref()) else {
                return false;
            };
            if stack.count >= threshold {
                return false;
            }
            let info = self.items().get(&stack.item);
            let protected = (keep_tools && is_tool(&stack.item))
                || (keep_food && info.is_some_and(|i| i.food.is_some()))
                || (keep_blocks && info.is_some_and(|i| i.block.is_some()));
            !protected
        }) else {
            return;
        };
        // One Throw click per sweep; the menu engine drops the whole stack.
        self.menu_click(Some(slot), 1, horizon_client::menu::Input::Throw);
    }

    // --------------------------------------------------- weather

    fn tick_weather(&mut self) {
        if !self.mod_menu.registry.is_enabled("weather_changer") {
            return;
        }
        let choice = self
            .mod_menu
            .registry
            .get_feature("weather_changer")
            .map_or(0, |f| f.get_enum_setting("weather"));
        let w = &mut self.world.weather;
        match choice {
            1 => {
                w.raining = true;
                w.thundering = false;
                w.rain_level = (1.0, 1.0);
                w.thunder_level = (0.0, 0.0);
            }
            2 => {
                w.raining = true;
                w.thundering = true;
                w.rain_level = (1.0, 1.0);
                w.thunder_level = (1.0, 1.0);
            }
            _ => {
                w.raining = false;
                w.thundering = false;
                w.rain_level = (0.0, 0.0);
                w.thunder_level = (0.0, 0.0);
            }
        }
    }

    // ------------------------------------------------------ auto tool

    /// Picks the fastest hotbar tool for `block_name`, as Player.getDestroySpeed
    /// would, and selects it.
    pub(super) fn mod_auto_tool(&mut self, block_name: &str) {
        if !self.mod_menu.registry.is_enabled("auto_tool") {
            return;
        }
        let mut best: Option<(usize, f32)> = None;
        for slot in 0..9 {
            let Some(stack) = self.inventory.get(slot).and_then(|s| s.as_ref()) else {
                continue;
            };
            if !is_tool(&stack.item) {
                continue;
            }
            let speed = self.items().mining_speed(Some(stack), block_name);
            if speed > 1.0 && best.is_none_or(|(_, b)| speed > b) {
                best = Some((slot, speed));
            }
        }
        if let Some((slot, _)) = best {
            if slot != self.selected {
                // Remember where to return when `swap_back` is on.
                if self.mod_menu.combat.prev_tool_slot.is_none() {
                    self.mod_menu.combat.prev_tool_slot = Some(self.selected);
                }
                self.selected = slot;
            }
        }
    }

    /// Auto Tool: returns to the previous slot once the block is gone.
    fn tick_auto_tool_swap_back(&mut self) {
        let swap_back = self.mod_menu.registry.is_enabled("auto_tool")
            && self
                .mod_menu
                .registry
                .get_feature("auto_tool")
                .is_some_and(|f| f.get_bool_setting("swap_back"));
        if !swap_back {
            self.mod_menu.combat.prev_tool_slot = None;
            return;
        }
        if self.mining.is_none() {
            if let Some(slot) = self.mod_menu.combat.prev_tool_slot.take() {
                self.selected = slot.min(8);
            }
        }
    }

    // --------------------------------------------- one-shot requests

    fn corz_add_waypoint(&mut self) {
        if !self.mod_menu.registry.is_enabled("waypoints") {
            return;
        }
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let n = self.mod_menu.config.waypoints.len();
        let name = format!("WP {}", n + 1);
        self.mod_menu.add_waypoint(
            &name,
            [p.position.x, p.position.y, p.position.z],
            self.dim.id(),
        );
    }

    fn corz_quick_friend(&mut self) {
        if !self.mod_menu.registry.is_enabled("quick_friend") {
            return;
        }
        let Some((index, _)) = self.mob_hit() else {
            return;
        };
        let Some(mob) = self.mobs.get(index) else {
            return;
        };
        if !mob.kind.id.ends_with("player") {
            return;
        }
        let Some(name) = mob.custom_name.as_deref() else {
            return;
        };
        let was_friend = self.mod_menu.registry.is_friend(name);
        if was_friend {
            self.mod_menu.registry.remove_friend(name);
            self.mod_menu.config.friends = self.mod_menu.registry.friends.clone();
            self.mod_menu.config.save();
            self.mod_menu.toast("Friends", &format!("Removed {name}"));
        } else {
            self.mod_menu.add_friend(name);
        }
    }
}

/// A rough tool classifier for Auto Tool: anything that speeds up mining.
fn is_tool(id: &str) -> bool {
    let id = id.trim_start_matches("minecraft:");
    id.contains("pickaxe")
        || id.contains("axe")
        || id.contains("shovel")
        || id.contains("hoe")
        || id.contains("sword")
}
