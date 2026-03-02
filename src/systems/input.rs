use bevy::ecs::message::MessageReader;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::networking::*;
use crate::types::*;

// --- Input & Camera ---

pub fn update_cursor_world_pos(
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    terrain: Res<TerrainHeightmap>,
    mut cursor_pos: ResMut<CursorWorldPos>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };
    if let Some(screen_pos) = window.cursor_position() {
        if let Ok(ray) = camera.viewport_to_world(cam_transform, screen_pos) {
            let dir_y = ray.direction.y;
            if dir_y.abs() > 0.001 {
                let mut plane_y = 0.0_f32;
                for _ in 0..3 {
                    let t = (plane_y - ray.origin.y) / dir_y;
                    if t > 0.0 {
                        let hit = ray.origin + *ray.direction * t;
                        let game_x = hit.x;
                        let game_y = -hit.z;
                        plane_y = terrain.height_at(game_x, game_y);
                    }
                }
                let t = (plane_y - ray.origin.y) / dir_y;
                if t > 0.0 {
                    let hit = ray.origin + *ray.direction * t;
                    cursor_pos.0 = Vec2::new(hit.x, -hit.z);
                }
            }
        }
    }
}

pub fn camera_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut projection)) = camera_q.single_mut() else {
        return;
    };

    let dt = time.delta_secs();
    let scale = match projection.as_ref() {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };
    let speed = CAMERA_SPEED * scale * dt;

    let forward_xz = Vec3::new(0.0, 0.0, -1.0);
    let right_xz = Vec3::new(1.0, 0.0, 0.0);

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        transform.translation += forward_xz * speed;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        transform.translation -= forward_xz * speed;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        transform.translation -= right_xz * speed;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        transform.translation += right_xz * speed;
    }

    for event in scroll_events.read() {
        let delta = match event.unit {
            MouseScrollUnit::Line => event.y * ZOOM_SPEED,
            MouseScrollUnit::Pixel => event.y * 0.001,
        };
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = (ortho.scale - delta).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
}

pub fn unit_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    cursor_pos: Res<CursorWorldPos>,
    mut drag: ResMut<DragSelect>,
    build_mode: Res<BuildMode>,
    dgun_mode: Res<DGunMode>,
    local_player: Res<LocalPlayer>,
    mut commands: Commands,
    units: Query<(Entity, &Transform, &Unit, &TeamOwned)>,
    selected: Query<Entity, With<Selected>>,
    mut sel_box_q: Query<(&mut Transform, &mut Visibility), (With<SelectionBox>, Without<Unit>)>,
) {
    if build_mode.active || dgun_mode.0 {
        return;
    }

    let world = cursor_pos.0;

    if mouse.just_pressed(MouseButton::Left) {
        drag.start = Some(world);
        drag.dragging = false;
    }

    if mouse.pressed(MouseButton::Left) {
        if let Some(start) = drag.start {
            if start.distance(world) > 5.0 {
                drag.dragging = true;
                if let Ok((mut tf, mut vis)) = sel_box_q.single_mut() {
                    let center = (start + world) / 2.0;
                    let size = (world - start).abs();
                    let world_center = game_pos(center.x, center.y, 1.0);
                    tf.translation = world_center;
                    tf.scale = Vec3::new(size.x, 1.0, size.y);
                    *vis = Visibility::Visible;
                }
            }
        }
    }

    if mouse.just_released(MouseButton::Left) && drag.start.is_some() {
        if !keyboard.pressed(KeyCode::ShiftLeft) && !keyboard.pressed(KeyCode::ShiftRight) {
            for entity in &selected {
                commands.entity(entity).remove::<Selected>();
            }
        }

        if drag.dragging {
            if let Some(start) = drag.start {
                let min = start.min(world);
                let max = start.max(world);
                for (entity, transform, _, team) in &units {
                    if team.0 != local_player.id {
                        continue;
                    }
                    let pos = game_xy(&transform.translation);
                    if pos.x >= min.x && pos.x <= max.x && pos.y >= min.y && pos.y <= max.y {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }
        } else {
            let mut closest: Option<(Entity, f32)> = None;
            for (entity, transform, _, team) in &units {
                if team.0 != local_player.id {
                    continue;
                }
                let dist = game_xy(&transform.translation).distance(world);
                if dist < 30.0 {
                    if closest.is_none() || dist < closest.unwrap().1 {
                        closest = Some((entity, dist));
                    }
                }
            }
            if let Some((entity, _)) = closest {
                commands.entity(entity).insert(Selected);
            }
        }

        if let Ok((_, mut vis)) = sel_box_q.single_mut() {
            *vis = Visibility::Hidden;
        }
        drag.start = None;
        drag.dragging = false;
    }
}

pub fn unit_commands(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<CursorWorldPos>,
    build_mode: Res<BuildMode>,
    dgun_mode: Res<DGunMode>,
    local_player: Res<LocalPlayer>,
    mut local_commands: ResMut<LocalCommands>,
    selected_units: Query<(Entity, &TeamOwned, &StableId, Option<&Commander>), (With<Selected>, With<Unit>)>,
    enemy_units: Query<(Entity, &Transform, &TeamOwned, &StableId), With<Unit>>,
    wreckages: Query<(Entity, &Transform, Option<&StableId>), With<Wreckage>>,
    map_features: Query<(Entity, &Transform, Option<&StableId>), With<MapFeature>>,
) {
    if build_mode.active || dgun_mode.0 {
        return;
    }

    if mouse.just_pressed(MouseButton::Right) {
        let world = cursor_pos.0;

        // Find target enemy
        let mut target_enemy: Option<(Entity, u64)> = None;
        for (enemy_entity, enemy_tf, enemy_team, enemy_sid) in &enemy_units {
            if enemy_team.0 == local_player.id {
                continue;
            }
            if game_xy(&enemy_tf.translation).distance(world) < 30.0 {
                target_enemy = Some((enemy_entity, enemy_sid.0));
                break;
            }
        }

        // Find reclaim target
        let mut target_reclaim: Option<u64> = None;
        if target_enemy.is_none() {
            for (_wreck_entity, wreck_tf, wreck_sid) in &wreckages {
                if game_xy(&wreck_tf.translation).distance(world) < 30.0 {
                    if let Some(sid) = wreck_sid {
                        target_reclaim = Some(sid.0);
                    }
                    break;
                }
            }
            if target_reclaim.is_none() {
                for (_feat_entity, feat_tf, feat_sid) in &map_features {
                    if game_xy(&feat_tf.translation).distance(world) < 30.0 {
                        if let Some(sid) = feat_sid {
                            target_reclaim = Some(sid.0);
                        }
                        break;
                    }
                }
            }
        }

        // Collect selected unit IDs
        let local_units: Vec<(u64, bool)> = selected_units.iter()
            .filter(|(_, t, _, _)| t.0 == local_player.id)
            .map(|(_, _, sid, cmd)| (sid.0, cmd.is_some()))
            .collect();

        if local_units.is_empty() {
            return;
        }

        if let Some((_, target_sid)) = target_enemy {
            let unit_ids: Vec<u64> = local_units.iter().map(|(id, _)| *id).collect();
            local_commands.commands.push(GameCommand::AttackUnits {
                unit_ids,
                target_id: target_sid,
            });
        } else if let Some(reclaim_sid) = target_reclaim {
            // Only commanders can reclaim; others move
            let commanders: Vec<u64> = local_units.iter().filter(|(_, is_cmd)| *is_cmd).map(|(id, _)| *id).collect();
            let non_commanders: Vec<u64> = local_units.iter().filter(|(_, is_cmd)| !*is_cmd).map(|(id, _)| *id).collect();

            for cid in commanders {
                local_commands.commands.push(GameCommand::Reclaim {
                    commander_id: cid,
                    target_id: reclaim_sid,
                });
            }
            if !non_commanders.is_empty() {
                local_commands.commands.push(GameCommand::MoveUnits {
                    unit_ids: non_commanders,
                    target: (world.x, world.y),
                });
            }
        } else {
            let unit_ids: Vec<u64> = local_units.iter().map(|(id, _)| *id).collect();
            local_commands.commands.push(GameCommand::MoveUnits {
                unit_ids,
                target: (world.x, world.y),
            });
        }
    }
}

pub fn build_mode_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut key_events: MessageReader<KeyboardInput>,
    mut build_mode: ResMut<BuildMode>,
    mut dgun_mode: ResMut<DGunMode>,
    local_player: Res<LocalPlayer>,
    selected_commanders: Query<&TeamOwned, (With<Selected>, With<Commander>)>,
) {
    let has_local_commander = selected_commanders.iter().any(|t| t.0 == local_player.id);
    if has_local_commander {
        if keyboard.just_pressed(KeyCode::Digit1) {
            build_mode.active = true;
            build_mode.building_type = Some(BuildingType::MetalExtractor);
            dgun_mode.0 = false;
        }
        if keyboard.just_pressed(KeyCode::Digit2) {
            build_mode.active = true;
            build_mode.building_type = Some(BuildingType::SolarCollector);
            dgun_mode.0 = false;
        }
        if keyboard.just_pressed(KeyCode::Digit3) {
            build_mode.active = true;
            build_mode.building_type = Some(BuildingType::Factory);
            dgun_mode.0 = false;
        }
        if keyboard.just_pressed(KeyCode::Digit4) {
            build_mode.active = true;
            build_mode.building_type = Some(BuildingType::LLT);
            dgun_mode.0 = false;
        }
        if keyboard.just_pressed(KeyCode::Digit5) {
            build_mode.active = true;
            build_mode.building_type = Some(BuildingType::Wall);
            dgun_mode.0 = false;
        }
        if keyboard.just_pressed(KeyCode::Digit6) {
            build_mode.active = true;
            build_mode.building_type = Some(BuildingType::RadarTower);
            dgun_mode.0 = false;
        }
    }
    for ev in key_events.read() {
        if ev.state.is_pressed() && ev.logical_key == bevy::input::keyboard::Key::Escape {
            build_mode.active = false;
            build_mode.building_type = None;
            dgun_mode.0 = false;
        }
    }
}

pub fn building_placement(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<CursorWorldPos>,
    terrain: Res<TerrainHeightmap>,
    mut build_mode: ResMut<BuildMode>,
    all_resources: Res<AllTeamResources>,
    local_player: Res<LocalPlayer>,
    mut local_commands: ResMut<LocalCommands>,
    metal_spots: Query<&Transform, With<MetalSpot>>,
    existing_buildings: Query<(&Transform, &Building), Without<MetalSpot>>,
    selected_commanders: Query<(&TeamOwned, &StableId), (With<Selected>, With<Commander>)>,
) {
    if !build_mode.active {
        return;
    }

    if mouse.just_pressed(MouseButton::Left) {
        let Some(btype) = build_mode.building_type else {
            return;
        };

        let mut place_pos = cursor_pos.0;

        let bs = btype.stats();
        let (metal_cost, energy_cost) = (bs.metal_cost, bs.energy_cost);

        let team_id = local_player.id as usize;
        if all_resources.teams[team_id].metal < metal_cost || all_resources.teams[team_id].energy < energy_cost {
            return;
        }

        if btype == BuildingType::MetalExtractor {
            let mut found_spot = false;
            for spot_tf in &metal_spots {
                let spot_pos = game_xy(&spot_tf.translation);
                if spot_pos.distance(place_pos) < EXTRACTOR_SNAP_RANGE {
                    place_pos = spot_pos;
                    found_spot = true;
                    break;
                }
            }
            if !found_spot {
                return;
            }
        }

        // Snap to build grid
        place_pos = snap_to_build_grid(place_pos, bs.size);

        let size = Vec2::new(bs.size.0, bs.size.1);

        // Check terrain flatness
        if !terrain.is_flat_enough(place_pos.x, place_pos.y, size.x, size.y) {
            return;
        }

        // Check no overlap with existing buildings
        let overlaps = existing_buildings.iter().any(|(b_tf, b)| {
            let b_pos = game_xy(&b_tf.translation);
            let b_stats = b.building_type.stats();
            let b_half = Vec2::new(b_stats.size.0 * 0.5, b_stats.size.1 * 0.5);
            let half = size * 0.5;
            (place_pos.x - half.x) < (b_pos.x + b_half.x)
                && (place_pos.x + half.x) > (b_pos.x - b_half.x)
                && (place_pos.y - half.y) < (b_pos.y + b_half.y)
                && (place_pos.y + half.y) > (b_pos.y - b_half.y)
        });
        if overlaps {
            return;
        }

        // Collect commander IDs for BuildTarget assignment
        let commander_ids: Vec<u64> = selected_commanders.iter()
            .filter(|(t, _)| t.0 == local_player.id)
            .map(|(_, sid)| sid.0)
            .collect();

        local_commands.commands.push(GameCommand::PlaceBuilding {
            building_type: btype.to_u8(),
            position: (place_pos.x, place_pos.y),
            commander_ids,
        });

        build_mode.active = false;
        build_mode.building_type = None;
    }
}

pub fn factory_queue_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    local_player: Res<LocalPlayer>,
    mut local_commands: ResMut<LocalCommands>,
    factories: Query<(&Factory, &Building, &TeamOwned, &StableId), With<Selected>>,
) {
    for (_factory, building, team, sid) in &factories {
        if !building.built || team.0 != local_player.id {
            continue;
        }

        let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
        let count = if shift { 5 } else { 1 };

        let mut queue_type = |unit_type: UnitType| {
            for _ in 0..count {
                local_commands.commands.push(GameCommand::QueueUnit {
                    factory_id: sid.0,
                    unit_type: unit_type.to_u8(),
                });
            }
        };

        if keyboard.just_pressed(KeyCode::KeyQ) { queue_type(UnitType::Scout); }
        if keyboard.just_pressed(KeyCode::KeyW) { queue_type(UnitType::Raider); }
        if keyboard.just_pressed(KeyCode::KeyE) { queue_type(UnitType::Tank); }
        if keyboard.just_pressed(KeyCode::KeyR) { queue_type(UnitType::Assault); }
        if keyboard.just_pressed(KeyCode::KeyT) { queue_type(UnitType::Artillery); }
    }
}

pub fn dgun_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<CursorWorldPos>,
    mut dgun_mode: ResMut<DGunMode>,
    mut build_mode: ResMut<BuildMode>,
    all_resources: Res<AllTeamResources>,
    local_player: Res<LocalPlayer>,
    mut local_commands: ResMut<LocalCommands>,
    selected_commanders: Query<(Entity, &Transform, &TeamOwned, &StableId), (With<Selected>, With<Commander>)>,
) {
    if keyboard.just_pressed(KeyCode::KeyG) {
        let has_local = selected_commanders.iter().any(|(_, _, t, _)| t.0 == local_player.id);
        if has_local {
            dgun_mode.0 = !dgun_mode.0;
            if dgun_mode.0 {
                build_mode.active = false;
                build_mode.building_type = None;
            }
        }
    }

    if !dgun_mode.0 {
        return;
    }

    if mouse.just_pressed(MouseButton::Left) {
        let team_id = local_player.id as usize;
        if all_resources.teams[team_id].energy < DGUN_ENERGY_COST {
            return;
        }

        for (_, cmd_tf, team, sid) in &selected_commanders {
            if team.0 != local_player.id {
                continue;
            }
            let cmd_pos = game_xy(&cmd_tf.translation);
            let target_pos = cursor_pos.0;
            let dist = cmd_pos.distance(target_pos);

            if dist > DGUN_RANGE {
                continue;
            }

            local_commands.commands.push(GameCommand::DGun {
                commander_id: sid.0,
                target_pos: (target_pos.x, target_pos.y),
            });

            break;
        }

        dgun_mode.0 = false;
    }
}
