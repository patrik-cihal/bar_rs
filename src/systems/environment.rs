use bevy::prelude::*;

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

// --- Fog of War ---

pub fn fog_of_war_system(
    player_units: Query<(&Transform, &SightRange), With<PlayerOwned>>,
    player_radars: Query<(&Transform, &RadarRangeComp, &Building), With<PlayerOwned>>,
    mut enemy_entities: Query<
        (Entity, &Transform, &mut Visibility, Option<&Wreckage>),
        (With<EnemyOwned>, Without<PlayerOwned>),
    >,
) {
    for (_entity, enemy_tf, mut vis, _wreckage) in &mut enemy_entities {
        let enemy_pos = game_xy(&enemy_tf.translation);
        let mut in_sight = false;
        let mut in_radar = false;

        for (friendly_tf, sight_range) in &player_units {
            let dist = game_xy(&friendly_tf.translation).distance(enemy_pos);
            if dist <= sight_range.0 {
                in_sight = true;
                break;
            }
        }

        if !in_sight {
            for (radar_tf, radar_range, building) in &player_radars {
                if !building.built {
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
