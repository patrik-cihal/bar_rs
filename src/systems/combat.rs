use bevy::prelude::*;

use crate::types::*;

// --- Combat ---

pub fn combat_system(
    mut commands: Commands,
    mut units: Query<(
        Entity,
        &mut Unit,
        &Transform,
        Option<&PlayerOwned>,
        Option<&EnemyOwned>,
        Option<&Commander>,
        Option<&Building>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    struct CombatInfo {
        entity: Entity,
        pos: Vec2,
        attack_damage: f32,
        attack_range: f32,
        min_attack_range: f32,
        cooldown_timer: f32,
        is_player: bool,
    }

    let mut infos: Vec<CombatInfo> = Vec::new();
    for (entity, unit, transform, player, _enemy, _commander, _building) in &units {
        infos.push(CombatInfo {
            entity,
            pos: game_xy(&transform.translation),
            attack_damage: unit.attack_damage,
            attack_range: unit.attack_range,
            min_attack_range: unit.min_attack_range,
            cooldown_timer: unit.cooldown_timer,
            is_player: player.is_some(),
        });
    }

    let mut shots: Vec<(Vec2, Entity, f32, bool)> = Vec::new();
    let mut cooldown_resets: Vec<Entity> = Vec::new();

    for info in &infos {
        if info.attack_damage == 0.0 || info.cooldown_timer > 0.0 {
            continue;
        }

        let mut nearest: Option<(Entity, f32)> = None;
        for other in &infos {
            if other.entity == info.entity || other.is_player == info.is_player {
                continue;
            }
            let dist = info.pos.distance(other.pos);
            if dist <= info.attack_range && dist >= info.min_attack_range {
                if nearest.is_none() || dist < nearest.unwrap().1 {
                    nearest = Some((other.entity, dist));
                }
            }
        }

        if let Some((target, _)) = nearest {
            shots.push((info.pos, target, info.attack_damage, info.is_player));
            cooldown_resets.push(info.entity);
        }
    }

    for (entity, mut unit, _, _, _, _, _) in &mut units {
        if unit.cooldown_timer > 0.0 {
            unit.cooldown_timer -= dt;
        }
        if cooldown_resets.contains(&entity) {
            unit.cooldown_timer = unit.attack_cooldown;
        }
    }

    for (from_pos, target, damage, is_player) in shots {
        let color = if is_player {
            Color::srgb(0.5, 0.8, 1.0)
        } else {
            Color::srgb(1.0, 0.6, 0.3)
        };
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(PROJECTILE_SIZE / 2.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::from(color) * 3.0,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(game_pos(from_pos.x, from_pos.y, 1.5)),
            Projectile {
                target,
                damage,
                speed: PROJECTILE_SPEED,
                is_dgun: false,
            },
        ));
    }

    // Collect death info before removing
    let mut deaths: Vec<(Entity, Vec2, bool, f32)> = Vec::new();
    for (entity, unit, transform, _player, _enemy, commander, _building) in &units {
        if unit.hp <= 0.0 {
            let metal_value = if commander.is_some() {
                250.0
            } else {
                unit.max_hp * 0.3
            };
            deaths.push((
                entity,
                game_xy(&transform.translation),
                commander.is_some(),
                metal_value,
            ));
        }
    }

    for (entity, pos, is_commander, metal_value) in &deaths {
        if *is_commander {
            for info in &infos {
                if info.entity == *entity {
                    continue;
                }
                let dist = info.pos.distance(*pos);
                if dist < COMMANDER_DEATH_RADIUS {
                    if let Ok((_, mut unit, _, _, _, _, _)) = units.get_mut(info.entity) {
                        unit.hp -= COMMANDER_DEATH_DAMAGE;
                    }
                }
            }

            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(10.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.8, 0.2, 0.8),
                    alpha_mode: AlphaMode::Blend,
                    emissive: LinearRgba::new(5.0, 4.0, 1.0, 1.0),
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(game_pos(pos.x, pos.y, 2.0)),
                DeathExplosion {
                    timer: 0.0,
                    max_radius: COMMANDER_DEATH_RADIUS,
                },
            ));
        }

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(15.0, 3.0, 15.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.4, 0.35),
                unlit: false,
                ..default()
            })),
            Transform::from_translation(game_pos(pos.x, pos.y, 0.1)),
            Wreckage {
                metal_value: *metal_value,
                decay_timer: WRECKAGE_DECAY_TIME,
            },
        ));

        commands.entity(*entity).despawn();
    }
}

pub fn projectile_system(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Transform, &Projectile)>,
    mut units: Query<(&mut Unit, Option<&Commander>), Without<Projectile>>,
    transforms: Query<&Transform, (With<Unit>, Without<Projectile>)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (proj_entity, mut proj_tf, projectile) in &mut projectiles {
        let Some(target_tf) = transforms.get(projectile.target).ok() else {
            commands.entity(proj_entity).despawn();
            continue;
        };

        let target_pos = game_xy(&target_tf.translation);
        let proj_pos = game_xy(&proj_tf.translation);
        let direction = (target_pos - proj_pos).normalize_or_zero();
        let move_dist = projectile.speed * dt;

        if proj_pos.distance(target_pos) <= move_dist + 5.0 {
            if projectile.is_dgun {
                if let Ok((_, commander)) = units.get(projectile.target) {
                    if commander.is_some() {
                        commands.entity(proj_entity).despawn();
                        continue;
                    }
                }
            }

            if let Ok((mut unit, _)) = units.get_mut(projectile.target) {
                unit.hp -= projectile.damage;
            }
            commands.entity(proj_entity).despawn();
        } else {
            proj_tf.translation.x += direction.x * move_dist;
            proj_tf.translation.z -= direction.y * move_dist; // game +Y = world -Z
        }
    }
}
