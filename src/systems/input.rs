use bevy::ecs::message::MessageReader;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::spawning::*;
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
        // Cast ray from camera and intersect with terrain surface
        // Iterative approach: intersect at estimated height, refine
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

    // Camera moves along the ground plane (XZ), maintaining its height/angle
    // W/Up = screen up = camera's local "forward" projected onto XZ
    // With tilted camera, forward has both -Z and -Y components;
    // we only want the XZ movement to keep the camera at constant angle
    let forward_xz = Vec3::new(0.0, 0.0, -1.0); // game north = world -Z
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
    mut commands: Commands,
    units: Query<(Entity, &Transform, &Unit), With<PlayerOwned>>,
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
                for (entity, transform, _) in &units {
                    let pos = game_xy(&transform.translation);
                    if pos.x >= min.x && pos.x <= max.x && pos.y >= min.y && pos.y <= max.y {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }
        } else {
            let mut closest: Option<(Entity, f32)> = None;
            for (entity, transform, _) in &units {
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
    mut commands: Commands,
    selected_units: Query<(Entity, Option<&Commander>), (With<Selected>, With<PlayerOwned>, With<Unit>)>,
    enemy_units: Query<(Entity, &Transform), (With<EnemyOwned>, With<Unit>)>,
    wreckages: Query<(Entity, &Transform), With<Wreckage>>,
    map_features: Query<(Entity, &Transform), With<MapFeature>>,
) {
    if build_mode.active || dgun_mode.0 {
        return;
    }

    if mouse.just_pressed(MouseButton::Right) {
        let world = cursor_pos.0;

        let mut target_enemy: Option<Entity> = None;
        for (enemy_entity, enemy_tf) in &enemy_units {
            if game_xy(&enemy_tf.translation).distance(world) < 30.0 {
                target_enemy = Some(enemy_entity);
                break;
            }
        }

        let mut target_reclaim: Option<Entity> = None;
        if target_enemy.is_none() {
            for (wreck_entity, wreck_tf) in &wreckages {
                if game_xy(&wreck_tf.translation).distance(world) < 30.0 {
                    target_reclaim = Some(wreck_entity);
                    break;
                }
            }
            if target_reclaim.is_none() {
                for (feat_entity, feat_tf) in &map_features {
                    if game_xy(&feat_tf.translation).distance(world) < 30.0 {
                        target_reclaim = Some(feat_entity);
                        break;
                    }
                }
            }
        }

        for (entity, commander) in &selected_units {
            if let Some(enemy) = target_enemy {
                commands
                    .entity(entity)
                    .insert(AttackTarget(enemy))
                    .remove::<MoveTarget>()
                    .remove::<ReclaimTarget>()
                    .remove::<BuildTarget>();
            } else if let Some(reclaim) = target_reclaim {
                if commander.is_some() {
                    commands
                        .entity(entity)
                        .insert(ReclaimTarget(reclaim))
                        .remove::<MoveTarget>()
                        .remove::<AttackTarget>()
                        .remove::<BuildTarget>();
                } else {
                    commands
                        .entity(entity)
                        .insert(MoveTarget(world))
                        .remove::<AttackTarget>()
                        .remove::<ReclaimTarget>()
                        .remove::<BuildTarget>();
                }
            } else {
                commands
                    .entity(entity)
                    .insert(MoveTarget(world))
                    .remove::<AttackTarget>()
                    .remove::<ReclaimTarget>()
                    .remove::<BuildTarget>();
            }
        }
    }
}

pub fn build_mode_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut build_mode: ResMut<BuildMode>,
    mut dgun_mode: ResMut<DGunMode>,
    selected_commanders: Query<(), (With<Selected>, With<Commander>, With<PlayerOwned>)>,
) {
    if !selected_commanders.is_empty() {
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
    if keyboard.just_pressed(KeyCode::Escape) {
        build_mode.active = false;
        build_mode.building_type = None;
        dgun_mode.0 = false;
    }
}

pub fn building_placement(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<CursorWorldPos>,
    mut build_mode: ResMut<BuildMode>,
    mut resources: ResMut<GameResources>,
    metal_spots: Query<&Transform, With<MetalSpot>>,
    selected_commanders: Query<Entity, (With<Selected>, With<PlayerOwned>, With<Commander>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    models: Res<ModelLibrary>,
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

        if resources.metal < metal_cost || resources.energy < energy_cost {
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

        resources.metal -= metal_cost;
        resources.energy -= energy_cost;

        let building_entity = spawn_building_entity(&mut commands, &mut meshes, &mut materials, place_pos, btype, true, false, &models);

        for entity in &selected_commanders {
            commands
                .entity(entity)
                .insert(BuildTarget(building_entity))
                .remove::<MoveTarget>()
                .remove::<AttackTarget>()
                .remove::<ReclaimTarget>();
        }

        build_mode.active = false;
        build_mode.building_type = None;
    }
}

pub fn factory_queue_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut factories: Query<(&mut Factory, &Building), (With<Selected>, With<PlayerOwned>)>,
) {
    for (mut factory, building) in &mut factories {
        if !building.built {
            continue;
        }

        let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
        let count = if shift { 5 } else { 1 };

        if keyboard.just_pressed(KeyCode::KeyQ) {
            for _ in 0..count {
                factory.queue.push(UnitType::Scout);
            }
        }
        if keyboard.just_pressed(KeyCode::KeyW) {
            for _ in 0..count {
                factory.queue.push(UnitType::Raider);
            }
        }
        if keyboard.just_pressed(KeyCode::KeyE) {
            for _ in 0..count {
                factory.queue.push(UnitType::Tank);
            }
        }
        if keyboard.just_pressed(KeyCode::KeyR) {
            for _ in 0..count {
                factory.queue.push(UnitType::Assault);
            }
        }
        if keyboard.just_pressed(KeyCode::KeyT) {
            for _ in 0..count {
                factory.queue.push(UnitType::Artillery);
            }
        }
    }
}

pub fn dgun_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<CursorWorldPos>,
    mut dgun_mode: ResMut<DGunMode>,
    mut build_mode: ResMut<BuildMode>,
    mut resources: ResMut<GameResources>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain: Res<TerrainHeightmap>,
    selected_commanders: Query<(Entity, &Transform), (With<Selected>, With<Commander>, With<PlayerOwned>)>,
) {
    if keyboard.just_pressed(KeyCode::KeyG) {
        if !selected_commanders.is_empty() {
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
        if resources.energy < DGUN_ENERGY_COST {
            return;
        }

        for (_, cmd_tf) in &selected_commanders {
            let cmd_pos = game_xy(&cmd_tf.translation);
            let target_pos = cursor_pos.0;
            let dist = cmd_pos.distance(target_pos);

            if dist > DGUN_RANGE {
                continue;
            }

            resources.energy -= DGUN_ENERGY_COST;

            let direction = (target_pos - cmd_pos).normalize_or_zero();
            let end_pos = cmd_pos + direction * DGUN_RANGE;

            let target_entity = commands
                .spawn((
                    Transform::from_translation(game_pos(end_pos.x, end_pos.y, terrain.height_at(end_pos.x, end_pos.y))),
                    Unit {
                        hp: 1.0,
                        max_hp: 1.0,
                        speed: 0.0,
                        attack_damage: 0.0,
                        attack_range: 0.0,
                        attack_cooldown: 999.0,
                        cooldown_timer: 0.0,
                        min_attack_range: 0.0,
                        radius: 0.0,
                    },
                    Visibility::Hidden,
                ))
                .id();

            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(4.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 1.0, 0.0),
                    emissive: LinearRgba::new(5.0, 5.0, 0.0, 1.0),
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(game_pos(cmd_pos.x, cmd_pos.y, terrain.height_at(cmd_pos.x, cmd_pos.y) + 1.5)),
                Projectile {
                    target: target_entity,
                    damage: 9999.0,
                    speed: PROJECTILE_SPEED * 1.5,
                    is_dgun: true,
                },
            ));

            break;
        }

        dgun_mode.0 = false;
    }
}
