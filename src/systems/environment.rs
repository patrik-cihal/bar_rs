use bevy::prelude::*;
use bevy::mesh::VertexAttributeValues;

use crate::types::*;

// --- Unit-Unit Collision ---

pub fn unit_collision_system(
    mut units: Query<(Entity, &mut Transform, &Unit), Without<Building>>,
) {
    // Collect positions first to avoid borrow issues
    let unit_data: Vec<(Entity, Vec2, f32)> = units
        .iter()
        .filter(|(_, _, u)| u.speed > 0.0)
        .map(|(e, tf, u)| (e, game_xy(&tf.translation), u.radius))
        .collect();

    for i in 0..unit_data.len() {
        for j in (i + 1)..unit_data.len() {
            let (e_a, pos_a, rad_a) = unit_data[i];
            let (e_b, pos_b, rad_b) = unit_data[j];
            let min_dist = rad_a + rad_b;
            let dist = pos_a.distance(pos_b);
            if dist < min_dist && dist > 0.1 {
                let push_dir = (pos_a - pos_b).normalize_or_zero();
                let overlap = (min_dist - dist) * 0.5;
                // Push A away from B
                if let Ok((_, mut tf_a, _)) = units.get_mut(e_a) {
                    tf_a.translation.x += push_dir.x * overlap;
                    tf_a.translation.z -= push_dir.y * overlap;
                }
                // Push B away from A
                if let Ok((_, mut tf_b, _)) = units.get_mut(e_b) {
                    tf_b.translation.x -= push_dir.x * overlap;
                    tf_b.translation.z += push_dir.y * overlap;
                }
            }
        }
    }
}

// --- Unit-Building Collision ---

pub fn building_collision_system(
    buildings: Query<(&Transform, &Building)>,
    mut units: Query<(&mut Transform, &Unit), Without<Building>>,
) {
    for (bld_tf, building) in &buildings {
        if !building.built {
            continue;
        }
        let bld_pos = game_xy(&bld_tf.translation);
        let bs = building.building_type.stats();
        let half_w = bs.size.0 / 2.0;
        let half_h = bs.size.1 / 2.0;

        for (mut unit_tf, unit) in &mut units {
            if unit.speed == 0.0 {
                continue;
            }
            let unit_pos = game_xy(&unit_tf.translation);
            let r = unit.radius;

            // Find closest point on building rect to unit center
            let closest_x = unit_pos.x.clamp(bld_pos.x - half_w, bld_pos.x + half_w);
            let closest_y = unit_pos.y.clamp(bld_pos.y - half_h, bld_pos.y + half_h);
            let closest = Vec2::new(closest_x, closest_y);

            let dist = unit_pos.distance(closest);
            if dist < r && dist > 0.01 {
                // Push unit out
                let push_dir = (unit_pos - closest).normalize_or_zero();
                let push_amount = r - dist;
                unit_tf.translation.x += push_dir.x * push_amount;
                unit_tf.translation.z -= push_dir.y * push_amount;
            } else if dist <= 0.01 {
                // Unit center is inside the building rect — push out along shortest axis
                let dx_left = (unit_pos.x - (bld_pos.x - half_w)).abs();
                let dx_right = (unit_pos.x - (bld_pos.x + half_w)).abs();
                let dy_bottom = (unit_pos.y - (bld_pos.y - half_h)).abs();
                let dy_top = (unit_pos.y - (bld_pos.y + half_h)).abs();
                let min_d = dx_left.min(dx_right).min(dy_bottom).min(dy_top);
                let (push_dir, push_amount) = if min_d == dx_left {
                    (Vec2::new(-1.0, 0.0), dx_left + r)
                } else if min_d == dx_right {
                    (Vec2::new(1.0, 0.0), dx_right + r)
                } else if min_d == dy_bottom {
                    (Vec2::new(0.0, -1.0), dy_bottom + r)
                } else {
                    (Vec2::new(0.0, 1.0), dy_top + r)
                };
                unit_tf.translation.x += push_dir.x * push_amount;
                unit_tf.translation.z -= push_dir.y * push_amount;
            }
        }
    }
}

// --- Terrain Follow ---

pub fn terrain_follow_system(
    terrain: Res<TerrainHeightmap>,
    mut units: Query<&mut Transform, With<Unit>>,
) {
    for mut tf in &mut units {
        let gpos = game_xy(&tf.translation);
        let h = terrain.height_at(gpos.x, gpos.y);
        tf.translation.y = h;
    }
}

// --- Fog of War ---

pub fn fog_of_war_system(
    local_player: Res<LocalPlayer>,
    friendly_units: Query<(&Transform, &SightRange, &TeamOwned)>,
    friendly_radars: Query<(&Transform, &RadarRangeComp, &Building, &TeamOwned)>,
    mut enemy_entities: Query<
        (Entity, &Transform, &mut Visibility, &TeamOwned, Option<&Wreckage>),
        With<Unit>,
    >,
) {
    let local_team = local_player.id;

    for (_entity, enemy_tf, mut vis, team, _wreckage) in &mut enemy_entities {
        if team.0 == local_team {
            continue; // don't hide our own units
        }

        let enemy_pos = game_xy(&enemy_tf.translation);
        let mut in_sight = false;
        let mut in_radar = false;

        for (friendly_tf, sight_range, friendly_team) in &friendly_units {
            if friendly_team.0 != local_team {
                continue;
            }
            let dist = game_xy(&friendly_tf.translation).distance(enemy_pos);
            if dist <= sight_range.0 {
                in_sight = true;
                break;
            }
        }

        if !in_sight {
            for (radar_tf, radar_range, building, radar_team) in &friendly_radars {
                if radar_team.0 != local_team || !building.built {
                    continue;
                }
                let dist = game_xy(&radar_tf.translation).distance(enemy_pos);
                if dist <= radar_range.0 {
                    in_radar = true;
                    break;
                }
            }
        }

        if in_sight || in_radar {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

// --- Fog Overlay Visualization ---

pub fn fog_overlay_system(
    local_player: Res<LocalPlayer>,
    all_units: Query<(&Transform, &SightRange, &TeamOwned)>,
    all_radars: Query<(&Transform, &RadarRangeComp, &Building, &TeamOwned)>,
    fog_query: Query<&Mesh3d, With<FogOverlay>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(mesh_handle) = fog_query.single() else {
        return;
    };
    let Some(mesh) = meshes.get_mut(&mesh_handle.0) else {
        return;
    };

    let local_team = local_player.id;
    let gs = FOG_GRID_SIZE;
    let cell_size = MAP_SIZE / (gs - 1) as f32;

    // Collect player vision sources
    let sight_sources: Vec<(Vec2, f32)> = all_units
        .iter()
        .filter(|(_, _, t)| t.0 == local_team)
        .map(|(tf, sr, _)| (game_xy(&tf.translation), sr.0))
        .collect();

    let radar_sources: Vec<(Vec2, f32)> = all_radars
        .iter()
        .filter(|(_, _, b, t)| t.0 == local_team && b.built)
        .map(|(tf, rr, _, _)| (game_xy(&tf.translation), rr.0))
        .collect();

    // Build new vertex colors
    let mut colors = Vec::with_capacity(gs * gs);
    for gy in 0..gs {
        for gx in 0..gs {
            let wx = gx as f32 * cell_size;
            let wy = gy as f32 * cell_size;
            let pos = Vec2::new(wx, wy);

            let mut alpha = 0.7_f32; // full fog

            // Check sight ranges — fully clear
            for &(src, range) in &sight_sources {
                let dist = src.distance(pos);
                if dist < range {
                    // Smooth edge: fade from 0 alpha at center to fog at range
                    let edge_width = 60.0;
                    let fade = ((range - dist) / edge_width).min(1.0);
                    alpha = alpha.min(0.7 * (1.0 - fade));
                }
            }

            // Check radar ranges — partial clear (dimmer fog)
            for &(src, range) in &radar_sources {
                let dist = src.distance(pos);
                if dist < range {
                    let edge_width = 80.0;
                    let fade = ((range - dist) / edge_width).min(1.0);
                    let radar_alpha = 0.7 * (1.0 - fade * 0.5); // radar only halves the fog
                    alpha = alpha.min(radar_alpha);
                }
            }

            colors.push([0.0_f32, 0.0, 0.0, alpha]);
        }
    }

    mesh.insert_attribute(
        Mesh::ATTRIBUTE_COLOR,
        VertexAttributeValues::Float32x4(colors),
    );
}
