use bevy::prelude::*;
use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};

use crate::types::*;

// --- Combat ---

pub fn combat_system(
    mut commands: Commands,
    mut units: Query<(
        Entity,
        &mut Unit,
        &Transform,
        &TeamOwned,
        Option<&Commander>,
        Option<&Building>,
        Option<&StableId>,
    )>,
    sight_units: Query<(&Transform, &SightRange, &TeamOwned), Without<Building>>,
    camera_q: Query<&Transform, (With<Camera3d>, Without<Unit>, Without<Building>)>,
    local_player: Res<LocalPlayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain: Res<TerrainHeightmap>,
    mut stable_id_map: ResMut<StableIdMap>,
    sounds: Res<SoundLibrary>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let local_team = local_player.id;
    let cam_pos = camera_q.single().map(|tf| game_xy(&tf.translation)).unwrap_or_default();

    // Collect friendly sight sources for visibility-gated audio
    let sight_sources: Vec<(Vec2, f32)> = sight_units
        .iter()
        .filter(|(_, _, t)| t.0 == local_team)
        .map(|(tf, sr, _)| (game_xy(&tf.translation), sr.0))
        .collect();
    let is_visible = |pos: Vec2| -> bool {
        sight_sources.iter().any(|(src, range)| src.distance(pos) <= *range)
    };

    struct CombatInfo {
        entity: Entity,
        pos: Vec2,
        attack_damage: f32,
        attack_range: f32,
        min_attack_range: f32,
        cooldown_timer: f32,
        team: u8,
        stable_id: Option<u64>,
        weapon_sound: &'static str,
    }

    let mut infos: Vec<CombatInfo> = Vec::new();
    for (entity, unit, transform, team, _commander, _building, sid) in &units {
        infos.push(CombatInfo {
            entity,
            pos: game_xy(&transform.translation),
            attack_damage: unit.attack_damage,
            attack_range: unit.attack_range,
            min_attack_range: unit.min_attack_range,
            cooldown_timer: unit.cooldown_timer,
            team: team.0,
            stable_id: sid.map(|s| s.0),
            weapon_sound: unit.weapon_sound,
        });
    }

    // Sort by StableId for deterministic target acquisition
    infos.sort_by_key(|i| i.stable_id.unwrap_or(u64::MAX));

    let mut shots: Vec<(Vec2, Entity, f32, u8, &str)> = Vec::new();
    let mut cooldown_resets: Vec<Entity> = Vec::new();

    for info in &infos {
        if info.attack_damage == 0.0 || info.cooldown_timer > 0.0 {
            continue;
        }

        let mut nearest: Option<(Entity, f32)> = None;
        for other in &infos {
            if other.entity == info.entity || other.team == info.team {
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
            shots.push((info.pos, target, info.attack_damage, info.team, info.weapon_sound));
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

    for (from_pos, target, damage, team, weapon_sound) in shots {
        let color = if team == 0 {
            Color::srgb(0.5, 0.8, 1.0)
        } else {
            Color::srgb(1.0, 0.6, 0.3)
        };
        let spawn_pos = game_pos(from_pos.x, from_pos.y, terrain.height_at(from_pos.x, from_pos.y) + 1.5);
        commands.spawn((
            Mesh3d(meshes.add(Capsule3d::new(1.5, 5.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::from(color) * 5.0,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(spawn_pos),
            Projectile {
                target,
                damage,
                speed: PROJECTILE_SPEED,
                is_dgun: false,
            },
        ));
        // Weapon fire sound (visibility-gated + distance-attenuated)
        if !weapon_sound.is_empty() && (team == local_team || is_visible(from_pos)) {
            let vol = distance_volume(0.5, cam_pos.distance(from_pos));
            if vol > 0.0 {
                if let Some(handle) = sounds.get(weapon_sound) {
                    commands.spawn((
                        AudioPlayer::new(handle.clone()),
                        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vol)),
                    ));
                }
            }
        }
        // Muzzle flash at shooter position
        let flash_color = if team == 0 {
            LinearRgba::new(2.0, 4.0, 8.0, 1.0)
        } else {
            LinearRgba::new(8.0, 4.0, 1.0, 1.0)
        };
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(3.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: flash_color,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(spawn_pos),
            MuzzleFlash { lifetime: 0.1 },
        ));
    }

    // Collect death info before removing
    let mut deaths: Vec<(Entity, Vec2, bool, f32, Option<u64>, &str)> = Vec::new();
    for (entity, unit, transform, _team, commander, _building, sid) in &units {
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
                sid.map(|s| s.0),
                unit.death_sound,
            ));
        }
    }

    for (entity, pos, is_commander, metal_value, sid, death_sound) in &deaths {
        // Death explosion sound (visibility-gated + distance-attenuated)
        if !death_sound.is_empty() && is_visible(*pos) {
            let base_vol = if *is_commander { 0.9 } else if *death_sound == "xplomed2" { 0.7 } else { 0.6 };
            let vol = distance_volume(base_vol, cam_pos.distance(*pos));
            if vol > 0.0 {
                if let Some(handle) = sounds.get(*death_sound) {
                    commands.spawn((
                        AudioPlayer::new(handle.clone()),
                        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vol)),
                    ));
                }
            }
        }
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
                Transform::from_translation(game_pos(pos.x, pos.y, terrain.height_at(pos.x, pos.y) + 2.0)),
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
            Transform::from_translation(game_pos(pos.x, pos.y, terrain.height_at(pos.x, pos.y) + 0.1)),
            Wreckage {
                metal_value: *metal_value,
                decay_timer: WRECKAGE_DECAY_TIME,
            },
        ));

        // Explosion particles — 8 debris sparks with deterministic spread
        let death_height = terrain.height_at(pos.x, pos.y) + 2.0;
        let particle_emissive = if deaths.iter().position(|d| d.0 == *entity).unwrap_or(0) % 2 == 0 {
            LinearRgba::new(1.0, 3.0, 5.0, 1.0) // cyan sparks
        } else {
            LinearRgba::new(5.0, 3.0, 0.5, 1.0) // orange sparks
        };
        let stable_seed = sid.unwrap_or(entity.to_bits());
        for i in 0..8u64 {
            let angle = (stable_seed.wrapping_add(i).wrapping_mul(2654435761)) as f32 / u32::MAX as f32 * std::f32::consts::TAU;
            let speed = 40.0 + (i as f32) * 8.0;
            let velocity = Vec3::new(angle.cos() * speed, 30.0 + (i as f32) * 5.0, angle.sin() * speed);
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(1.5))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    emissive: particle_emissive,
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(game_pos(pos.x, pos.y, death_height)),
                ExplosionParticle { velocity, lifetime: 0.6 + (i as f32) * 0.05 },
            ));
        }
        // Death flash
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(8.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(10.0, 8.0, 3.0, 1.0),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(game_pos(pos.x, pos.y, death_height)),
            MuzzleFlash { lifetime: 0.15 },
        ));

        if let Some(sid) = sid {
            stable_id_map.remove(*sid);
        }
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
            // Orient capsule in travel direction
            let look_target = proj_tf.translation + Vec3::new(direction.x, 0.0, -direction.y) * 10.0;
            proj_tf.look_at(look_target, Vec3::Y);
        }
    }
}
