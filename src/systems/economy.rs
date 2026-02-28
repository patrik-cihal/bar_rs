use bevy::prelude::*;

use crate::spawning::*;
use crate::types::*;

// --- Economy ---

pub fn resource_production(
    mut resources: ResMut<GameResources>,
    extractors: Query<&Building, (With<MetalExtractor>, With<PlayerOwned>)>,
    solars: Query<&Building, (With<SolarCollector>, With<PlayerOwned>)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let mut metal_income = 0.0;
    let mut energy_income = 0.0;

    for building in &extractors {
        if building.built {
            metal_income += EXTRACTOR_INCOME;
        }
    }

    for building in &solars {
        if building.built {
            energy_income += SOLAR_INCOME;
        }
    }

    resources.metal += metal_income * dt;
    resources.energy += energy_income * dt;
    resources.metal_income = metal_income;
    resources.energy_income = energy_income;
}

pub fn factory_production(
    mut commands: Commands,
    mut factories: Query<(&mut Factory, &Building, &Transform, Option<&PlayerOwned>, Option<&EnemyOwned>)>,
    mut resources: ResMut<GameResources>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    models: Res<ModelLibrary>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut factory, building, transform, player, _enemy) in &mut factories {
        if !building.built {
            continue;
        }

        let is_player = player.is_some();

        if is_player {
            if factory.queue.is_empty() {
                continue;
            }

            let unit_type = factory.queue[0];
            let us = unit_type.stats();

            if factory.produce_timer == 0.0 {
                if resources.metal < us.metal_cost || resources.energy < us.energy_cost {
                    continue;
                }
                resources.metal -= us.metal_cost;
                resources.energy -= us.energy_cost;
                factory.current_build_time = us.build_time;
            }

            factory.produce_timer += dt;

            if factory.produce_timer >= factory.current_build_time {
                factory.produce_timer = 0.0;
                factory.queue.remove(0);

                let factory_pos = game_xy(&transform.translation);
                let spawn_pos = factory_pos + Vec2::new(0.0, -60.0);
                spawn_unit(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    spawn_pos,
                    true,
                    Some(unit_type),
                    &models,
                );
            }
        } else {
            factory.produce_timer += dt;
            if factory.produce_timer >= 5.0 {
                factory.produce_timer = 0.0;
                let factory_pos = game_xy(&transform.translation);
                let spawn_pos = factory_pos + Vec2::new(0.0, -60.0);
                spawn_unit(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    spawn_pos,
                    false,
                    Some(UnitType::Tank),
                    &models,
                );
            }
        }
    }
}

pub fn reclaim_system(
    mut commands: Commands,
    mut resources: ResMut<GameResources>,
    reclaimers: Query<(Entity, &Transform, Option<&ReclaimTarget>), (With<Commander>, With<PlayerOwned>)>,
    mut wreckages: Query<(Entity, &Transform, &mut Wreckage), Without<Commander>>,
    mut map_features: Query<(Entity, &Transform, &mut MapFeature), (Without<Commander>, Without<Wreckage>)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (_cmd_entity, cmd_tf, reclaim_target) in &reclaimers {
        let Some(ReclaimTarget(target)) = reclaim_target else {
            continue;
        };
        let cmd_pos = game_xy(&cmd_tf.translation);

        if let Ok((wreck_entity, wreck_tf, mut wreckage)) = wreckages.get_mut(*target) {
            let dist = cmd_pos.distance(game_xy(&wreck_tf.translation));
            if dist <= RECLAIM_RANGE {
                let reclaim_rate = wreckage.metal_value / RECLAIM_TIME;
                let reclaimed = reclaim_rate * dt;
                resources.metal += reclaimed;
                wreckage.metal_value -= reclaimed;
                if wreckage.metal_value <= 0.0 {
                    commands.entity(wreck_entity).despawn();
                }
            }
        } else if let Ok((feat_entity, feat_tf, mut feature)) = map_features.get_mut(*target) {
            let dist = cmd_pos.distance(game_xy(&feat_tf.translation));
            if dist <= RECLAIM_RANGE {
                let reclaim_rate = feature.metal_value / RECLAIM_TIME;
                let reclaimed = reclaim_rate * dt;
                resources.metal += reclaimed;
                feature.metal_value -= reclaimed;
                if feature.metal_value <= 0.0 {
                    commands.entity(feat_entity).despawn();
                }
            }
        }
    }
}

pub fn wreckage_decay_system(
    mut commands: Commands,
    mut wreckages: Query<(Entity, &mut Wreckage)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (entity, mut wreckage) in &mut wreckages {
        wreckage.decay_timer -= dt;
        if wreckage.decay_timer <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
