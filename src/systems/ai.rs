use bevy::prelude::*;

use crate::networking::{CommandBuffer, GameCommand, NetRole};
use crate::types::*;

// --- AI Configuration ---

const AI_BASE: Vec2 = Vec2::new(1700.0, 1700.0);

/// Priority order for metal spot selection (enemy spots first, then middle, then player)
const AI_SPOT_PRIORITY: [usize; 10] = [8, 9, 6, 7, 5, 4, 3, 2, 1, 0];

/// Scripted build order for the AI commander
const SCRIPTED_BUILD: [BuildingType; 10] = [
    BuildingType::MetalExtractor,
    BuildingType::MetalExtractor,
    BuildingType::SolarCollector,
    BuildingType::SolarCollector,
    BuildingType::Factory,
    BuildingType::MetalExtractor,
    BuildingType::LLT,
    BuildingType::RadarTower,
    BuildingType::SolarCollector,
    BuildingType::LLT,
];

/// Cumulative weights for unit production (out of 100)
/// Scout=10%, Raider=25%, Tank=35%, Assault=20%, Artillery=10%
const UNIT_WEIGHTS: [(UnitType, u64); 5] = [
    (UnitType::Scout, 10),
    (UnitType::Raider, 35),
    (UnitType::Tank, 70),
    (UnitType::Assault, 90),
    (UnitType::Artillery, 100),
];

const ECONOMY_INTERVAL: u64 = 15;
const PRODUCTION_INTERVAL: u64 = 15;
const MILITARY_INTERVAL: u64 = 30;
const ATTACK_HP_THRESHOLD: f32 = 600.0;
const FORCE_SWEEP_TICKS: u64 = 1800; // 60 seconds at 30Hz

// --- AI State ---

#[derive(Resource)]
pub struct AiState {
    last_tick: u64,
    build_index: usize,
    build_count: usize,
    last_economy_tick: u64,
    last_production_tick: u64,
    last_military_tick: u64,
    last_attack_tick: u64,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            last_tick: u64::MAX,
            build_index: 0,
            build_count: 0,
            last_economy_tick: 0,
            last_production_tick: 0,
            last_military_tick: 0,
            last_attack_tick: 0,
        }
    }
}

// --- Deterministic RNG ---

fn ai_rand(tick: u64, salt: u64) -> u64 {
    let mut x = tick.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(salt);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

// --- Helper Functions ---

/// Find the first unclaimed metal spot for team 1
fn find_unclaimed_spot(
    extractors: &Query<(&Transform, &TeamOwned), With<MetalExtractor>>,
) -> Option<Vec2> {
    for &idx in &AI_SPOT_PRIORITY {
        let (sx, sy) = METAL_SPOT_POSITIONS[idx];
        let spot = Vec2::new(sx, sy);

        let claimed = extractors.iter().any(|(tf, team)| {
            team.0 == 1 && game_xy(&tf.translation).distance(spot) < EXTRACTOR_SNAP_RANGE
        });

        if !claimed {
            return Some(spot);
        }
    }
    None
}

/// Find a valid position for a non-extractor building using spiral placement
fn find_spiral_pos(
    build_count: usize,
    size: (f32, f32),
    terrain: &TerrainHeightmap,
    buildings: &Query<(&Transform, &Building, &TeamOwned, &StableId)>,
) -> Option<Vec2> {
    let half = Vec2::new(size.0 * 0.5, size.1 * 0.5);

    for attempt in 0..20 {
        let idx = build_count + attempt;
        let angle = idx as f32 * 1.2;
        let radius = 60.0 + idx as f32 * 20.0;
        let raw = AI_BASE + Vec2::new(angle.cos() * radius, angle.sin() * radius);
        let pos = snap_to_build_grid(raw, size);

        // Bounds check
        if pos.x - half.x < 0.0
            || pos.x + half.x > MAP_SIZE
            || pos.y - half.y < 0.0
            || pos.y + half.y > MAP_SIZE
        {
            continue;
        }

        // Terrain flatness
        if !terrain.is_flat_enough(pos.x, pos.y, size.0, size.1) {
            continue;
        }

        // No overlap with existing buildings
        let overlaps = buildings.iter().any(|(b_tf, b, _, _)| {
            let b_pos = game_xy(&b_tf.translation);
            let b_size = b.building_type.stats().size;
            let b_half = Vec2::new(b_size.0 * 0.5, b_size.1 * 0.5);
            (pos.x - half.x) < (b_pos.x + b_half.x)
                && (pos.x + half.x) > (b_pos.x - b_half.x)
                && (pos.y - half.y) < (b_pos.y + b_half.y)
                && (pos.y + half.y) > (b_pos.y - b_half.y)
        });

        if !overlaps {
            return Some(pos);
        }
    }
    None
}

/// Pick a unit type using weighted random selection
fn pick_unit_type(tick: u64, salt: u64) -> UnitType {
    let r = ai_rand(tick, salt) % 100;
    for &(utype, threshold) in &UNIT_WEIGHTS {
        if r < threshold {
            return utype;
        }
    }
    UnitType::Tank
}

// --- Main AI System ---

pub fn ai_system(
    mut ai: ResMut<AiState>,
    mut command_buffer: ResMut<CommandBuffer>,
    net_role: Res<NetRole>,
    all_resources: Res<AllTeamResources>,
    terrain: Res<TerrainHeightmap>,
    commanders: Query<
        (&Transform, &StableId, &TeamOwned, Option<&BuildTarget>),
        With<Commander>,
    >,
    factories: Query<(&Factory, &Building, &StableId, &TeamOwned)>,
    all_units: Query<
        (
            &Transform,
            &Unit,
            &StableId,
            &TeamOwned,
            Option<&MoveTarget>,
            Option<&AttackTarget>,
            Option<&Commander>,
        ),
        Without<Building>,
    >,
    buildings: Query<(&Transform, &Building, &TeamOwned, &StableId)>,
    extractors: Query<(&Transform, &TeamOwned), With<MetalExtractor>>,
) {
    if !matches!(*net_role, NetRole::Singleplayer) {
        return;
    }

    let tick = command_buffer.current_tick;
    if tick == ai.last_tick {
        return;
    }
    ai.last_tick = tick;

    let mut cmds: Vec<GameCommand> = Vec::new();
    let team_res = &all_resources.teams[1];

    // --- 1. Economy: Commander building ---
    if tick == 0 || tick.wrapping_sub(ai.last_economy_tick) >= ECONOMY_INTERVAL {
        ai.last_economy_tick = tick;

        // Find idle AI commander (team 1, no BuildTarget)
        let ai_cmd = commanders
            .iter()
            .find(|(_, _, t, bt)| t.0 == 1 && bt.is_none());

        if let Some((_, cmd_sid, _, _)) = ai_cmd {
            let sid = cmd_sid.0;

            if ai.build_index < SCRIPTED_BUILD.len() {
                // Scripted build order
                let btype = SCRIPTED_BUILD[ai.build_index];
                let bs = btype.stats();

                if team_res.metal >= bs.metal_cost && team_res.energy >= bs.energy_cost {
                    let place_pos = if btype == BuildingType::MetalExtractor {
                        find_unclaimed_spot(&extractors)
                    } else {
                        find_spiral_pos(ai.build_count, bs.size, &terrain, &buildings)
                    };

                    if let Some(pos) = place_pos {
                        cmds.push(GameCommand::PlaceBuilding {
                            building_type: btype.to_u8(),
                            position: (pos.x, pos.y),
                            commander_ids: vec![sid],
                        });
                        ai.build_index += 1;
                        if btype != BuildingType::MetalExtractor {
                            ai.build_count += 1;
                        }
                    } else if btype == BuildingType::MetalExtractor {
                        // No unclaimed spots left, skip
                        ai.build_index += 1;
                    }
                }
            } else {
                // Reactive economy
                ai_reactive_economy(
                    sid,
                    team_res,
                    &terrain,
                    &extractors,
                    &buildings,
                    &factories,
                    &mut cmds,
                    &mut ai.build_count,
                );
            }
        }
    }

    // --- 2. Factory Production ---
    if tick >= ai.last_production_tick + PRODUCTION_INTERVAL {
        ai.last_production_tick = tick;

        let mut salt = 0u64;
        for (factory, building, f_sid, team) in &factories {
            if team.0 != 1 || !building.built || factory.queue.len() >= 3 {
                continue;
            }

            let utype = pick_unit_type(tick, salt);
            salt += 1;

            cmds.push(GameCommand::QueueUnit {
                factory_id: f_sid.0,
                unit_type: utype.to_u8(),
            });
        }
    }

    // --- 3. Military Attack ---
    if tick >= ai.last_military_tick + MILITARY_INTERVAL {
        ai.last_military_tick = tick;

        // Collect idle AI military units (team 1, not commander, has speed, no orders)
        let mut idle_units: Vec<(Vec2, u64, f32)> = Vec::new();
        for (tf, unit, sid, team, mt, at, cmd) in &all_units {
            if team.0 != 1 || cmd.is_some() || unit.speed == 0.0 {
                continue;
            }
            if mt.is_some() || at.is_some() {
                continue;
            }
            idle_units.push((game_xy(&tf.translation), sid.0, unit.hp));
        }

        let total_hp: f32 = idle_units.iter().map(|(_, _, hp)| hp).sum();
        let force_sweep = tick.wrapping_sub(ai.last_attack_tick) >= FORCE_SWEEP_TICKS;

        if (total_hp >= ATTACK_HP_THRESHOLD || force_sweep) && !idle_units.is_empty() {
            // Find center of idle army
            let sum: Vec2 = idle_units.iter().map(|(p, _, _)| *p).sum();
            let army_center = sum / idle_units.len() as f32;

            // Find nearest player entity (unit or building)
            let mut best_target: Option<(f32, u64)> = None;

            for (tf, _, sid, team, _, _, _) in &all_units {
                if team.0 != 0 {
                    continue;
                }
                let dist = game_xy(&tf.translation).distance(army_center);
                if best_target.is_none() || dist < best_target.unwrap().0 {
                    best_target = Some((dist, sid.0));
                }
            }
            for (tf, _, team, sid) in &buildings {
                if team.0 != 0 {
                    continue;
                }
                let dist = game_xy(&tf.translation).distance(army_center);
                if best_target.is_none() || dist < best_target.unwrap().0 {
                    best_target = Some((dist, sid.0));
                }
            }

            let unit_ids: Vec<u64> = idle_units.iter().map(|(_, sid, _)| *sid).collect();

            if let Some((_, target_sid)) = best_target {
                cmds.push(GameCommand::AttackUnits {
                    unit_ids,
                    target_id: target_sid,
                });
                ai.last_attack_tick = tick;
            } else if force_sweep {
                cmds.push(GameCommand::MoveUnits {
                    unit_ids,
                    target: (200.0, 200.0),
                });
                ai.last_attack_tick = tick;
            }
        }
    }

    // Inject AI commands into command buffer
    if !cmds.is_empty() {
        command_buffer
            .pending
            .entry((tick, 1))
            .or_default()
            .extend(cmds);
    }
}

fn ai_reactive_economy(
    commander_sid: u64,
    team_res: &GameResources,
    terrain: &TerrainHeightmap,
    extractors: &Query<(&Transform, &TeamOwned), With<MetalExtractor>>,
    buildings: &Query<(&Transform, &Building, &TeamOwned, &StableId)>,
    factories: &Query<(&Factory, &Building, &StableId, &TeamOwned)>,
    cmds: &mut Vec<GameCommand>,
    build_count: &mut usize,
) {
    // Priority 1: Build extractor if unclaimed spot available
    if let Some(spot) = find_unclaimed_spot(extractors) {
        let bs = BuildingType::MetalExtractor.stats();
        if team_res.metal >= bs.metal_cost && team_res.energy >= bs.energy_cost {
            cmds.push(GameCommand::PlaceBuilding {
                building_type: BuildingType::MetalExtractor.to_u8(),
                position: (spot.x, spot.y),
                commander_ids: vec![commander_sid],
            });
            return;
        }
    }

    // Priority 2: Build solar if energy low
    if team_res.energy < 300.0 || team_res.energy_income < team_res.metal_income * 10.0 {
        let bs = BuildingType::SolarCollector.stats();
        if team_res.metal >= bs.metal_cost {
            if let Some(pos) = find_spiral_pos(*build_count, bs.size, terrain, buildings) {
                cmds.push(GameCommand::PlaceBuilding {
                    building_type: BuildingType::SolarCollector.to_u8(),
                    position: (pos.x, pos.y),
                    commander_ids: vec![commander_sid],
                });
                *build_count += 1;
                return;
            }
        }
    }

    // Priority 3: Build factory if resources are high
    if team_res.metal > 800.0 && team_res.energy > 1500.0 {
        let factory_count = factories.iter().filter(|(_, _, _, t)| t.0 == 1).count();
        if factory_count < 3 {
            let bs = BuildingType::Factory.stats();
            if team_res.metal >= bs.metal_cost && team_res.energy >= bs.energy_cost {
                if let Some(pos) = find_spiral_pos(*build_count, bs.size, terrain, buildings) {
                    cmds.push(GameCommand::PlaceBuilding {
                        building_type: BuildingType::Factory.to_u8(),
                        position: (pos.x, pos.y),
                        commander_ids: vec![commander_sid],
                    });
                    *build_count += 1;
                    return;
                }
            }
        }
    }

    // Priority 4: Build LLT for defense
    if team_res.metal > 200.0 {
        let bs = BuildingType::LLT.stats();
        if team_res.metal >= bs.metal_cost && team_res.energy >= bs.energy_cost {
            if let Some(pos) = find_spiral_pos(*build_count, bs.size, terrain, buildings) {
                cmds.push(GameCommand::PlaceBuilding {
                    building_type: BuildingType::LLT.to_u8(),
                    position: (pos.x, pos.y),
                    commander_ids: vec![commander_sid],
                });
                *build_count += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_rand_deterministic() {
        assert_eq!(ai_rand(42, 7), ai_rand(42, 7));
        assert_ne!(ai_rand(42, 7), ai_rand(42, 8));
        assert_ne!(ai_rand(42, 7), ai_rand(43, 7));
    }

    #[test]
    fn pick_unit_type_deterministic() {
        let a = pick_unit_type(100, 0);
        let b = pick_unit_type(100, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn pick_unit_type_varied() {
        let mut types = std::collections::HashSet::new();
        for tick in 0..1000 {
            types.insert(std::mem::discriminant(&pick_unit_type(tick, 0)));
        }
        assert!(
            types.len() >= 3,
            "should produce at least 3 different unit types"
        );
    }

    #[test]
    fn ai_rand_no_bias() {
        // Verify reasonable distribution across 10000 samples
        let mut buckets = [0u32; 10];
        for i in 0..10000u64 {
            let r = ai_rand(i, 0) % 10;
            buckets[r as usize] += 1;
        }
        for (i, &count) in buckets.iter().enumerate() {
            assert!(
                count > 500 && count < 1500,
                "bucket {} has {} (expected ~1000)",
                i,
                count
            );
        }
    }
}
