use bevy::prelude::*;

use crate::types::*;

/// Spawn a unit. If `unit_type` is None, spawns a Commander.
pub fn spawn_unit(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec2,
    team: u8,
    unit_type: Option<UnitType>,
    models: &ModelLibrary,
    next_stable_id: &mut NextStableId,
    stable_id_map: &mut StableIdMap,
) -> Entity {
    let is_commander = unit_type.is_none();

    let s = if is_commander {
        &COMMANDER_STATS
    } else {
        unit_type.unwrap().stats()
    };

    let world_pos = game_pos(pos.x, pos.y, 0.5);

    let unit_component = Unit {
        hp: s.hp,
        max_hp: s.hp,
        speed: s.speed,
        attack_damage: s.attack_damage,
        attack_range: s.attack_range,
        attack_cooldown: s.attack_cooldown,
        cooldown_timer: 0.0,
        min_attack_range: s.min_attack_range,
        radius: s.radius,
    };

    let sid = StableId(next_stable_id.0);
    next_stable_id.0 += 1;

    let mut entity = if let Some(model_handle) = models.get(s.model_file, team) {
        // Use 3D model from ModelLibrary
        let mut e = commands.spawn((
            SceneRoot(model_handle.clone()),
            Transform::from_translation(world_pos)
                .with_scale(Vec3::splat(s.model_scale)),
            unit_component,
            SightRange(s.sight_range),
            TeamOwned(team),
            sid,
        ));
        if is_commander {
            e.insert(CommanderWalkAnim { phase: 0.0, active: false });
        }
        e
    } else {
        // Fallback: cylinder mesh
        let color = if team == 0 {
            Color::srgb(0.3, 0.5, 0.9)
        } else {
            Color::srgb(0.9, 0.3, 0.3)
        };
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(s.radius, 8.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                unlit: false,
                ..default()
            })),
            Transform::from_translation(world_pos),
            unit_component,
            SightRange(s.sight_range),
            TeamOwned(team),
            sid,
        ))
    };

    if is_commander {
        entity.insert(Commander);
    } else {
        match unit_type.unwrap() {
            UnitType::Scout => { entity.insert(Scout); }
            UnitType::Raider => { entity.insert((Raider, VehicleAnim)); }
            UnitType::Tank => { entity.insert((Tank, VehicleAnim)); }
            UnitType::Assault => { entity.insert((Assault, VehicleAnim)); }
            UnitType::Artillery => {
                entity.insert((Artillery, BipedWalkAnim { phase: 0.0, active: false }));
            }
        }
    }

    // Health bar (child entities) — flat quads above unit
    let bar_offset_z = -(s.radius + 8.0); // "above" in game coords = -Z in world
    entity.with_children(|parent| {
        parent
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(30.0, 0.1, 4.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.2, 0.2),
                    unlit: true,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.3, bar_offset_z),
                HealthBarBg,
            ))
            .with_children(|bg| {
                bg.spawn((
                    Mesh3d(meshes.add(Cuboid::new(30.0, 0.1, 4.0))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.1, 0.9, 0.1),
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.1, 0.0),
                    HealthBarFill,
                ));
            });
    });

    let entity_id = entity.id();
    stable_id_map.insert(sid.0, entity_id);
    entity_id
}

pub fn spawn_building_entity(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec2,
    building_type: BuildingType,
    team: u8,
    pre_built: bool,
    models: &ModelLibrary,
    next_stable_id: &mut NextStableId,
    stable_id_map: &mut StableIdMap,
) -> Entity {
    let bs = building_type.stats();
    let size = Vec2::new(bs.size.0, bs.size.1);

    let world_pos = game_pos(pos.x, pos.y, 0.0);

    let sid = StableId(next_stable_id.0);
    next_stable_id.0 += 1;

    let mut entity = if let Some(model_handle) = models.get(bs.model_file, team) {
        // Use 3D model from ModelLibrary
        commands.spawn((
            SceneRoot(model_handle.clone()),
            Transform::from_translation(world_pos)
                .with_scale(Vec3::splat(bs.model_scale)),
            Building {
                building_type,
                built: pre_built,
                build_progress: if pre_built { bs.build_time } else { 0.0 },
                build_time: bs.build_time,
            },
            Unit {
                hp: bs.hp,
                max_hp: bs.hp,
                speed: 0.0,
                attack_damage: bs.attack_damage,
                attack_range: bs.attack_range,
                attack_cooldown: bs.attack_cooldown,
                cooldown_timer: 0.0,
                min_attack_range: 0.0,
                radius: 0.0,
            },
            SightRange(bs.sight_range),
            TeamOwned(team),
            sid,
        ))
    } else {
        // Fallback: cuboid mesh
        let color = match building_type {
            BuildingType::MetalExtractor => if team == 0 { Color::srgb(0.3, 0.3, 0.8) } else { Color::srgb(0.8, 0.3, 0.3) },
            BuildingType::SolarCollector => if team == 0 { Color::srgb(0.8, 0.8, 0.2) } else { Color::srgb(0.8, 0.5, 0.2) },
            BuildingType::Factory => if team == 0 { Color::srgb(0.2, 0.6, 0.2) } else { Color::srgb(0.6, 0.2, 0.2) },
            BuildingType::LLT => if team == 0 { Color::srgb(0.4, 0.4, 0.7) } else { Color::srgb(0.7, 0.4, 0.4) },
            BuildingType::Wall => if team == 0 { Color::srgb(0.5, 0.5, 0.5) } else { Color::srgb(0.6, 0.4, 0.4) },
            BuildingType::RadarTower => if team == 0 { Color::srgb(0.3, 0.7, 0.7) } else { Color::srgb(0.7, 0.5, 0.3) },
        };

        let alpha = if pre_built { 1.0 } else { 0.5 };
        let final_color = color.with_alpha(alpha);

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, 10.0, size.y))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: final_color,
                alpha_mode: if !pre_built { AlphaMode::Blend } else { AlphaMode::Opaque },
                unlit: false,
                ..default()
            })),
            Transform::from_translation(world_pos),
            Building {
                building_type,
                built: pre_built,
                build_progress: if pre_built { bs.build_time } else { 0.0 },
                build_time: bs.build_time,
            },
            Unit {
                hp: bs.hp,
                max_hp: bs.hp,
                speed: 0.0,
                attack_damage: bs.attack_damage,
                attack_range: bs.attack_range,
                attack_cooldown: bs.attack_cooldown,
                cooldown_timer: 0.0,
                min_attack_range: 0.0,
                radius: 0.0,
            },
            SightRange(bs.sight_range),
            TeamOwned(team),
            sid,
        ))
    };

    match building_type {
        BuildingType::MetalExtractor => {
            entity.insert(MetalExtractor);
        }
        BuildingType::SolarCollector => {
            entity.insert(SolarCollector);
        }
        BuildingType::Factory => {
            entity.insert(Factory {
                queue: Vec::new(),
                produce_timer: 0.0,
                current_build_time: 0.0,
            });
        }
        BuildingType::LLT => {
            entity.insert(LightLaserTower);
        }
        BuildingType::Wall => {
            entity.insert(DragonTeeth);
        }
        BuildingType::RadarTower => {
            entity.insert((RadarTower, RadarRangeComp(RADAR_RANGE)));
        }
    }

    // Health bar for buildings
    let bar_z = -(size.y / 2.0 + 8.0);
    let building_id = entity.id();
    entity.with_children(|parent| {
        parent
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(30.0, 0.1, 4.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.2, 0.2),
                    unlit: true,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.3, bar_z),
                HealthBarBg,
            ))
            .with_children(|bg| {
                bg.spawn((
                    Mesh3d(meshes.add(Cuboid::new(30.0, 0.1, 4.0))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.1, 0.9, 0.1),
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.1, 0.0),
                    HealthBarFill,
                ));
            });
    });

    stable_id_map.insert(sid.0, building_id);
    building_id
}
