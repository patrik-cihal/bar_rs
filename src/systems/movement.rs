use bevy::prelude::*;

use crate::types::*;

// --- Movement ---

pub fn unit_movement(
    mut commands: Commands,
    mut units: Query<(Entity, &mut Transform, &Unit, Option<&MoveTarget>, Option<&AttackTarget>, Option<&ReclaimTarget>, Option<&BuildTarget>, Option<&mut Path>)>,
    wreck_positions: Query<&Transform, (With<Wreckage>, Without<Unit>)>,
    feat_positions: Query<&Transform, (With<MapFeature>, Without<Unit>, Without<Wreckage>)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    let positions: Vec<(Entity, Vec2)> = units
        .iter()
        .map(|(e, tf, _, _, _, _, _, _)| (e, game_xy(&tf.translation)))
        .collect();

    let pos_map: std::collections::HashMap<Entity, Vec2> =
        positions.into_iter().collect();

    let mut moves: Vec<(Entity, Vec2)> = Vec::new();
    let mut remove_move_target: Vec<Entity> = Vec::new();
    let mut remove_path: Vec<Entity> = Vec::new();

    for (entity, transform, unit, move_target, attack_target, reclaim_target, build_target, mut path_opt) in &mut units {
        if unit.speed == 0.0 {
            continue;
        }

        let pos = game_xy(&transform.translation);

        // If unit has a path with waypoints, follow the next waypoint
        if let Some(ref mut path) = path_opt {
            if !path.waypoints.is_empty() {
                let next_wp = path.waypoints[0];
                let dist_to_wp = pos.distance(next_wp);
                if dist_to_wp <= BUILD_GRID_SIZE * 0.5 {
                    path.waypoints.remove(0);
                    // If more waypoints remain, move toward the new next one
                    if !path.waypoints.is_empty() {
                        let new_wp = path.waypoints[0];
                        let direction = (new_wp - pos).normalize_or_zero();
                        moves.push((entity, direction * unit.speed * dt));
                    }
                    // If waypoints exhausted, fall through to direct movement below
                    if path.waypoints.is_empty() {
                        remove_path.push(entity);
                    }
                    continue;
                } else {
                    let direction = (next_wp - pos).normalize_or_zero();
                    moves.push((entity, direction * unit.speed * dt));
                    continue;
                }
            }
        }

        // No path / path exhausted — use direct movement (existing behavior)
        if let Some(BuildTarget(target_entity)) = build_target {
            if let Some(&tpos) = pos_map.get(target_entity) {
                if pos.distance(tpos) > BUILD_RANGE * 0.9 {
                    let direction = (tpos - pos).normalize_or_zero();
                    moves.push((entity, direction * unit.speed * dt));
                }
            } else {
                remove_move_target.push(entity);
            }
        } else if let Some(AttackTarget(target_entity)) = attack_target {
            if let Some(&tpos) = pos_map.get(target_entity) {
                if pos.distance(tpos) > unit.attack_range * 0.9 {
                    let direction = (tpos - pos).normalize_or_zero();
                    moves.push((entity, direction * unit.speed * dt));
                }
            } else {
                remove_move_target.push(entity);
            }
        } else if let Some(ReclaimTarget(target_entity)) = reclaim_target {
            let target_pos = wreck_positions
                .get(*target_entity)
                .map(|tf| game_xy(&tf.translation))
                .or_else(|_| feat_positions.get(*target_entity).map(|tf| game_xy(&tf.translation)));
            if let Ok(tpos) = target_pos {
                if pos.distance(tpos) > RECLAIM_RANGE {
                    let direction = (tpos - pos).normalize_or_zero();
                    moves.push((entity, direction * unit.speed * dt));
                }
            } else {
                remove_move_target.push(entity);
            }
        } else if let Some(MoveTarget(target)) = move_target {
            if pos.distance(*target) > 5.0 {
                let direction = (*target - pos).normalize_or_zero();
                moves.push((entity, direction * unit.speed * dt));
            } else {
                remove_move_target.push(entity);
            }
        }
    }

    for (entity, movement) in moves {
        if let Ok((_, mut transform, _, _, _, _, _, _)) = units.get_mut(entity) {
            transform.translation.x += movement.x;
            transform.translation.z -= movement.y; // game +Y = world -Z
        }
    }

    for entity in remove_path {
        commands.entity(entity).remove::<Path>();
    }
    for entity in remove_move_target {
        commands.entity(entity).remove::<MoveTarget>();
        commands.entity(entity).remove::<BuildTarget>();
    }
}

pub fn building_construction(
    mut commands: Commands,
    mut buildings: Query<(Entity, &mut Building, &mut Transform)>,
    commanders: Query<(Entity, &Transform, Option<&BuildTarget>), (With<Commander>, Without<Building>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (building_entity, mut building, mut building_tf) in &mut buildings {
        if building.built {
            continue;
        }

        let building_pos = game_xy(&building_tf.translation);

        // Check if any commander with BuildTarget pointing to this building is in range
        let mut builder_world_pos: Option<Vec3> = None;
        let mut builder_entity: Option<Entity> = None;
        for (cmd_entity, cmd_tf, build_target) in &commanders {
            if let Some(BuildTarget(target)) = build_target {
                if *target == building_entity {
                    let cmd_pos = game_xy(&cmd_tf.translation);
                    if cmd_pos.distance(building_pos) < BUILD_RANGE {
                        builder_world_pos = Some(cmd_tf.translation);
                        builder_entity = Some(cmd_entity);
                        break;
                    }
                }
            }
        }

        if let Some(cmd_world_pos) = builder_world_pos {
            building.build_progress += dt;

            if building.build_progress >= building.build_time {
                building.built = true;
                if let Some(cmd_entity) = builder_entity {
                    commands.entity(cmd_entity).remove::<BuildTarget>();
                }
            }

            // Spawn nano particles from commander to building (~8 per second)
            let t = time.elapsed_secs();
            let spawn_interval = 0.12;
            let phase = (t / spawn_interval) as u32;
            if (t % spawn_interval) < dt {
                let cmd_world = cmd_world_pos + Vec3::new(0.0, 3.0, 0.0);
                let bld_world = building_tf.translation + Vec3::new(0.0, 2.0, 0.0);
                let offset = Vec3::new(
                    (phase as f32 * 37.0).sin() * 3.0,
                    (phase as f32 * 53.0).cos() * 2.0,
                    (phase as f32 * 71.0).sin() * 3.0,
                );
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(1.2))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgba(0.3, 1.0, 0.5, 0.8),
                        emissive: LinearRgba::new(0.5, 2.0, 0.8, 1.0),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_translation(cmd_world + offset),
                    NanoParticle {
                        target: bld_world,
                        speed: 200.0,
                        lifetime: 2.0,
                    },
                ));
            }
        }

        // Scale Y based on build progress — building "rises from the ground"
        let progress = (building.build_progress / building.build_time).min(1.0);
        let base_scale = building_tf.scale.x; // uniform scale was set at spawn
        let y_scale = base_scale * (0.05 + progress * 0.95);
        building_tf.scale.y = y_scale;
    }
}

pub fn nano_particle_system(
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Transform, &mut NanoParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut tf, mut particle) in &mut particles {
        particle.lifetime -= dt;
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let direction = (particle.target - tf.translation).normalize_or_zero();
        let dist = tf.translation.distance(particle.target);
        let move_dist = particle.speed * dt;

        if dist <= move_dist + 1.0 {
            commands.entity(entity).despawn();
        } else {
            tf.translation += direction * move_dist;
        }
    }
}
