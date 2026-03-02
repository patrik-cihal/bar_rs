use bevy::prelude::*;
use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};

use crate::spawning::*;
use crate::types::*;

// --- Economy ---

pub fn resource_production(
    mut all_resources: ResMut<AllTeamResources>,
    extractors: Query<(&Building, &TeamOwned), With<MetalExtractor>>,
    solars: Query<(&Building, &TeamOwned), With<SolarCollector>>,
    commanders: Query<&TeamOwned, With<Commander>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for team_res in all_resources.teams.iter_mut() {
        team_res.metal_income = 0.0;
        team_res.energy_income = 0.0;
    }

    // Commander passive income (like BAR)
    for team in &commanders {
        all_resources.teams[team.0 as usize].metal_income += COMMANDER_METAL_INCOME;
        all_resources.teams[team.0 as usize].energy_income += COMMANDER_ENERGY_INCOME;
    }

    for (building, team) in &extractors {
        if building.built {
            all_resources.teams[team.0 as usize].metal_income += EXTRACTOR_INCOME;
        }
    }

    for (building, team) in &solars {
        if building.built {
            all_resources.teams[team.0 as usize].energy_income += SOLAR_INCOME;
        }
    }

    for team_res in all_resources.teams.iter_mut() {
        team_res.metal += team_res.metal_income * dt;
        team_res.energy += team_res.energy_income * dt;
    }
}

pub fn factory_production(
    mut commands: Commands,
    mut factories: Query<(&mut Factory, &Building, &Transform, &TeamOwned)>,
    mut all_resources: ResMut<AllTeamResources>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    models: Res<ModelLibrary>,
    sounds: Res<SoundLibrary>,
    local_player: Res<LocalPlayer>,
    mut next_stable_id: ResMut<NextStableId>,
    mut stable_id_map: ResMut<StableIdMap>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut factory, building, transform, team) in &mut factories {
        if !building.built {
            continue;
        }

        let team_id = team.0 as usize;

        // Queue-based production (AI fills the queue via QueueUnit commands)
        if factory.queue.is_empty() {
            continue;
        }

        let unit_type = factory.queue[0];
        let us = unit_type.stats();

        if factory.produce_timer == 0.0 {
            if all_resources.teams[team_id].metal < us.metal_cost || all_resources.teams[team_id].energy < us.energy_cost {
                continue;
            }
            all_resources.teams[team_id].metal -= us.metal_cost;
            all_resources.teams[team_id].energy -= us.energy_cost;
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
                team.0,
                Some(unit_type),
                &models,
                &mut next_stable_id,
                &mut stable_id_map,
            );
            if team.0 == local_player.id {
                if let Some(handle) = sounds.get("unitready") {
                    commands.spawn((
                        AudioPlayer::new(handle.clone()),
                        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.5)),
                    ));
                }
            }
        }
    }
}

pub fn reclaim_system(
    mut commands: Commands,
    mut all_resources: ResMut<AllTeamResources>,
    reclaimers: Query<(Entity, &Transform, &TeamOwned, Option<&ReclaimTarget>), With<Commander>>,
    mut wreckages: Query<(Entity, &Transform, &mut Wreckage), Without<Commander>>,
    mut map_features: Query<(Entity, &Transform, &mut MapFeature), (Without<Commander>, Without<Wreckage>)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (_cmd_entity, cmd_tf, team, reclaim_target) in &reclaimers {
        let Some(ReclaimTarget(target)) = reclaim_target else {
            continue;
        };
        let cmd_pos = game_xy(&cmd_tf.translation);
        let team_id = team.0 as usize;

        if let Ok((wreck_entity, wreck_tf, mut wreckage)) = wreckages.get_mut(*target) {
            let dist = cmd_pos.distance(game_xy(&wreck_tf.translation));
            if dist <= RECLAIM_RANGE {
                let reclaim_rate = wreckage.metal_value / RECLAIM_TIME;
                let reclaimed = reclaim_rate * dt;
                all_resources.teams[team_id].metal += reclaimed;
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
                all_resources.teams[team_id].metal += reclaimed;
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
