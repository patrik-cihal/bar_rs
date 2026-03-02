use bevy::prelude::*;

use crate::types::*;


// --- Helper Functions ---

/// Interpolate a walk cycle value from keyframe data.
/// `keyframes` is a slice of (phase_0_to_1, value_degrees) pairs.
fn walk_lerp(keyframes: &[(f32, f32)], phase: f32) -> f32 {
    if keyframes.is_empty() {
        return 0.0;
    }
    let p = phase.fract();
    let mut prev = keyframes.last().unwrap();
    for kf in keyframes {
        if kf.0 >= p {
            let span = if kf.0 > prev.0 {
                kf.0 - prev.0
            } else {
                1.0 - prev.0 + kf.0
            };
            let t = if span > 0.001 {
                if kf.0 >= prev.0 {
                    (p - prev.0) / span
                } else {
                    ((p - prev.0 + 1.0) % 1.0) / span
                }
            } else {
                0.0
            };
            return prev.1 + (kf.1 - prev.1) * t;
        }
        prev = kf;
    }
    prev.1
}

fn deg(d: f32) -> f32 {
    d.to_radians()
}

// ============================================================
// Commander Walk Animation
// ============================================================

pub fn commander_animation_system(
    time: Res<Time>,
    mut commanders: Query<
        (Entity, &mut Transform, &mut CommanderWalkAnim, Option<&MoveTarget>, Option<&AttackTarget>, Option<&ReclaimTarget>, Option<&BuildTarget>),
        With<Commander>,
    >,
    target_transforms: Query<&GlobalTransform, Without<Commander>>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
    mut transform_query: Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    for (cmd_entity, mut tf, mut walk_anim, move_target, attack_target, reclaim_target, build_target) in &mut commanders {
        let is_moving = move_target.is_some() || attack_target.is_some() || reclaim_target.is_some() || build_target.is_some();
        walk_anim.active = is_moving;

        if is_moving {
            walk_anim.phase += dt * 2.5;
            if walk_anim.phase > 1.0 {
                walk_anim.phase -= 1.0;
            }
        } else {
            walk_anim.phase = 0.0;
        }

        // Determine facing direction
        let pos = game_xy(&tf.translation);
        let face_toward_entity = |entity: Entity| -> Vec2 {
            if let Ok(target_tf) = target_transforms.get(entity) {
                let tpos = target_tf.translation();
                (game_xy(&tpos) - pos).normalize_or_zero()
            } else {
                Vec2::ZERO
            }
        };
        let face_dir = if let Some(MoveTarget(target)) = move_target {
            (*target - pos).normalize_or_zero()
        } else if let Some(AttackTarget(e)) = attack_target {
            face_toward_entity(*e)
        } else if let Some(ReclaimTarget(e)) = reclaim_target {
            face_toward_entity(*e)
        } else if let Some(BuildTarget(e)) = build_target {
            face_toward_entity(*e)
        } else {
            Vec2::ZERO
        };

        if face_dir.length_squared() > 0.01 {
            let facing_angle = f32::atan2(face_dir.x, -face_dir.y);
            tf.rotation = Quat::from_rotation_y(facing_angle);
        }

        let phase = walk_anim.phase;
        animate_commander_pieces(
            cmd_entity,
            &children_query,
            &name_query,
            &mut transform_query,
            phase,
            is_moving,
            t,
        );
    }
}

fn animate_commander_pieces(
    entity: Entity,
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
    transform_query: &mut Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    phase: f32,
    is_moving: bool,
    time: f32,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            if let Ok(name) = name_query.get(child) {
                apply_commander_piece_anim(child, name.as_str(), transform_query, phase, is_moving, time);
            }
            animate_commander_pieces(child, children_query, name_query, transform_query, phase, is_moving, time);
        }
    }
}

fn apply_commander_piece_anim(
    entity: Entity,
    name: &str,
    transform_query: &mut Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    phase: f32,
    is_moving: bool,
    time: f32,
) {
    let Ok(mut tf) = transform_query.get_mut(entity) else {
        return;
    };

    if !is_moving {
        match name {
            "dish" => {
                tf.rotation = Quat::from_rotation_y(time * 2.0);
            }
            "pelvis" => {
                let bob = (time * 1.5).sin() * 0.3;
                tf.translation.y = 31.0 + bob;
                tf.rotation = Quat::IDENTITY;
            }
            "lthigh" | "rthigh" => {
                tf.rotation = Quat::from_rotation_x(deg(-20.0));
            }
            "lleg" | "rleg" => {
                tf.rotation = Quat::from_rotation_x(deg(22.0));
            }
            "lfoot" | "rfoot" => {
                tf.rotation = Quat::IDENTITY;
            }
            "luparm" | "ruparm" => {
                tf.rotation = Quat::IDENTITY;
            }
            "head" | "torso" => {
                tf.rotation = Quat::IDENTITY;
            }
            _ => {}
        }
        return;
    }

    match name {
        "pelvis" => {
            let bob = walk_lerp(
                &[(0.0, -2.5), (0.25, -1.5), (0.5, -2.5), (0.75, -1.5)],
                phase,
            );
            let yaw = walk_lerp(
                &[(0.0, -5.0), (0.25, 0.0), (0.5, 5.0), (0.75, 0.0)],
                phase,
            );
            let roll = walk_lerp(
                &[(0.0, -4.0), (0.25, 0.0), (0.5, 4.0), (0.75, 0.0)],
                phase,
            );
            tf.translation.y = 31.0 + bob;
            tf.rotation = Quat::from_rotation_y(deg(yaw)) * Quat::from_rotation_z(deg(roll));
        }
        "lthigh" => {
            let pitch = walk_lerp(
                &[
                    (0.0, -50.0), (0.125, -35.0), (0.25, 10.0), (0.375, 20.0),
                    (0.5, 15.0), (0.625, 0.0), (0.75, -30.0), (0.875, -50.0),
                ],
                phase,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "rthigh" => {
            let pitch = walk_lerp(
                &[
                    (0.0, -50.0), (0.125, -35.0), (0.25, 10.0), (0.375, 20.0),
                    (0.5, 15.0), (0.625, 0.0), (0.75, -30.0), (0.875, -50.0),
                ],
                (phase + 0.5) % 1.0,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "lleg" => {
            let pitch = walk_lerp(
                &[
                    (0.0, 55.0), (0.125, 30.0), (0.25, 10.0), (0.375, 5.0),
                    (0.5, 20.0), (0.625, 60.0), (0.75, 90.0), (0.875, 70.0),
                ],
                phase,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "rleg" => {
            let pitch = walk_lerp(
                &[
                    (0.0, 55.0), (0.125, 30.0), (0.25, 10.0), (0.375, 5.0),
                    (0.5, 20.0), (0.625, 60.0), (0.75, 90.0), (0.875, 70.0),
                ],
                (phase + 0.5) % 1.0,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "lfoot" => {
            let pitch = walk_lerp(
                &[(0.0, -30.0), (0.25, 5.0), (0.5, 10.0), (0.75, -20.0)],
                phase,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "rfoot" => {
            let pitch = walk_lerp(
                &[(0.0, -30.0), (0.25, 5.0), (0.5, 10.0), (0.75, -20.0)],
                (phase + 0.5) % 1.0,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "luparm" => {
            let pitch = walk_lerp(
                &[(0.0, 15.0), (0.25, -5.0), (0.5, -10.0), (0.75, 15.0)],
                phase,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "ruparm" => {
            let pitch = walk_lerp(
                &[(0.0, 15.0), (0.25, -5.0), (0.5, -10.0), (0.75, 15.0)],
                (phase + 0.5) % 1.0,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "head" => {
            let yaw = walk_lerp(
                &[(0.0, -5.0), (0.25, 0.0), (0.5, 5.0), (0.75, 0.0)],
                phase,
            );
            tf.rotation = Quat::from_rotation_y(deg(yaw));
        }
        "dish" => {
            tf.rotation = Quat::from_rotation_y(time * 3.0);
        }
        "torso" => {
            let yaw = walk_lerp(
                &[(0.0, 3.0), (0.25, 0.0), (0.5, -3.0), (0.75, 0.0)],
                phase,
            );
            tf.rotation = Quat::from_rotation_y(deg(yaw));
        }
        _ => {}
    }
}

// ============================================================
// Artillery (armham) Walk Animation
// ============================================================

pub fn biped_walk_animation_system(
    time: Res<Time>,
    mut bipeds: Query<
        (Entity, &mut BipedWalkAnim, Option<&MoveTarget>),
        (With<Unit>, With<Artillery>, Without<Commander>),
    >,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
    mut transform_query: Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    for (entity, mut walk_anim, move_target) in &mut bipeds {
        let is_moving = move_target.is_some();
        walk_anim.active = is_moving;

        if is_moving {
            walk_anim.phase += dt * 2.0; // slightly slower than commander
            if walk_anim.phase > 1.0 {
                walk_anim.phase -= 1.0;
            }
        } else {
            walk_anim.phase = 0.0;
        }

        let phase = walk_anim.phase;
        animate_armham_pieces(entity, &children_query, &name_query, &mut transform_query, phase, is_moving, t);
    }
}

fn animate_armham_pieces(
    entity: Entity,
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
    transform_query: &mut Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    phase: f32,
    is_moving: bool,
    time: f32,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            if let Ok(name) = name_query.get(child) {
                apply_armham_piece_anim(child, name.as_str(), transform_query, phase, is_moving, time);
            }
            animate_armham_pieces(child, children_query, name_query, transform_query, phase, is_moving, time);
        }
    }
}

fn apply_armham_piece_anim(
    entity: Entity,
    name: &str,
    transform_query: &mut Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    phase: f32,
    is_moving: bool,
    _time: f32,
) {
    let Ok(mut tf) = transform_query.get_mut(entity) else {
        return;
    };

    if !is_moving {
        // Idle pose
        match name {
            "pelvis" => { tf.rotation = Quat::IDENTITY; }
            "lthigh" | "rthigh" => { tf.rotation = Quat::from_rotation_x(deg(-15.0)); }
            "lleg" | "rleg" => { tf.rotation = Quat::from_rotation_x(deg(15.0)); }
            "lfoot" | "rfoot" => { tf.rotation = Quat::IDENTITY; }
            "torso" => { tf.rotation = Quat::IDENTITY; }
            _ => {}
        }
        return;
    }

    // armham walk cycle from BOS data (simplified)
    match name {
        "pelvis" => {
            let bob = walk_lerp(
                &[(0.0, -0.9), (0.25, -1.9), (0.5, -1.4), (0.75, -1.2)],
                phase,
            );
            let yaw = walk_lerp(
                &[(0.0, -6.0), (0.25, -10.0), (0.5, 6.0), (0.75, 10.0)],
                phase,
            );
            let roll = walk_lerp(
                &[(0.0, -3.0), (0.25, -5.0), (0.5, 3.0), (0.75, 5.0)],
                phase,
            );
            tf.translation.y = 15.6 + bob;
            tf.rotation = Quat::from_rotation_y(deg(yaw)) * Quat::from_rotation_z(deg(roll));
        }
        "lthigh" => {
            let pitch = walk_lerp(
                &[
                    (0.0, -57.0), (0.125, -40.0), (0.25, -27.0), (0.375, 0.0),
                    (0.5, 27.0), (0.625, 10.0), (0.75, -26.0), (0.875, -50.0),
                ],
                phase,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "rthigh" => {
            let pitch = walk_lerp(
                &[
                    (0.0, -57.0), (0.125, -40.0), (0.25, -27.0), (0.375, 0.0),
                    (0.5, 27.0), (0.625, 10.0), (0.75, -26.0), (0.875, -50.0),
                ],
                (phase + 0.5) % 1.0,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "lleg" => {
            let pitch = walk_lerp(
                &[
                    (0.0, 50.0), (0.125, 30.0), (0.25, 15.0), (0.375, 5.0),
                    (0.5, 20.0), (0.625, 55.0), (0.75, 74.0), (0.875, 60.0),
                ],
                phase,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "rleg" => {
            let pitch = walk_lerp(
                &[
                    (0.0, 50.0), (0.125, 30.0), (0.25, 15.0), (0.375, 5.0),
                    (0.5, 20.0), (0.625, 55.0), (0.75, 74.0), (0.875, 60.0),
                ],
                (phase + 0.5) % 1.0,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "lfoot" => {
            let pitch = walk_lerp(
                &[(0.0, -50.0), (0.25, 0.0), (0.5, 26.0), (0.75, -30.0)],
                phase,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "rfoot" => {
            let pitch = walk_lerp(
                &[(0.0, -50.0), (0.25, 0.0), (0.5, 26.0), (0.75, -30.0)],
                (phase + 0.5) % 1.0,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        "torso" => {
            // Counter-rotation and slight pitch
            let yaw = walk_lerp(
                &[(0.0, 8.0), (0.25, 15.0), (0.5, -8.0), (0.75, -15.0)],
                phase,
            );
            let pitch = walk_lerp(
                &[(0.0, -5.0), (0.25, -7.0), (0.5, -5.0), (0.75, -3.0)],
                phase,
            );
            tf.rotation = Quat::from_rotation_y(deg(yaw)) * Quat::from_rotation_x(deg(pitch));
        }
        "larm" | "rarm" => {
            // Arms swing opposite to legs
            let offset = if name == "rarm" { 0.5 } else { 0.0 };
            let pitch = walk_lerp(
                &[(0.0, 10.0), (0.25, -5.0), (0.5, -10.0), (0.75, 10.0)],
                (phase + offset) % 1.0,
            );
            tf.rotation = Quat::from_rotation_x(deg(pitch));
        }
        _ => {}
    }
}

// ============================================================
// Vehicle Turret Animation (tanks, raiders, assault)
// ============================================================

pub fn vehicle_animation_system(
    units: Query<
        (Entity, &Transform, Option<&AttackTarget>, Option<&MoveTarget>),
        (With<VehicleAnim>, With<Unit>, Without<Commander>, Without<Building>),
    >,
    target_transforms: Query<&GlobalTransform, With<Unit>>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
    mut transform_query: Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    time: Res<Time>,
) {
    let t = time.elapsed_secs();

    for (entity, unit_tf, attack_target, move_target) in &units {
        let unit_pos = game_xy(&unit_tf.translation);

        // Get unit's facing angle to compute relative turret heading
        let (_, unit_rot_angle, _) = unit_tf.rotation.to_euler(EulerRot::YXZ);

        // Find the target direction for turret aiming (relative to unit facing)
        let turret_yaw = if let Some(AttackTarget(target_entity)) = attack_target {
            if let Ok(target_gtf) = target_transforms.get(*target_entity) {
                let target_pos = game_xy(&target_gtf.translation());
                let dir = (target_pos - unit_pos).normalize_or_zero();
                if dir.length_squared() > 0.01 {
                    let world_angle = f32::atan2(dir.x, -dir.y);
                    // Turret angle is relative to unit body
                    world_angle - unit_rot_angle
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            // Idle: slow turret scan left-right
            (t * 0.5).sin() * 0.5
        };

        // Is moving? For wheel spin
        let is_moving = move_target.is_some();

        // Traverse children and animate turret, wheels
        animate_vehicle_pieces(
            entity,
            &children_query,
            &name_query,
            &mut transform_query,
            turret_yaw,
            is_moving,
            t,
        );
    }
}

fn animate_vehicle_pieces(
    entity: Entity,
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
    transform_query: &mut Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    turret_yaw: f32,
    is_moving: bool,
    time: f32,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            if let Ok(name) = name_query.get(child) {
                let n = name.as_str();
                if let Ok(mut tf) = transform_query.get_mut(child) {
                    match n {
                        "turret" => {
                            tf.rotation = Quat::from_rotation_y(turret_yaw);
                        }
                        // Wheels spin when moving (armbull has lwheel/rwheel)
                        "lwheel" | "rwheel" | "lbwheel" | "rbwheel" | "lfwheel" | "rfwheel" => {
                            if is_moving {
                                tf.rotation = Quat::from_rotation_x(time * 5.0);
                            }
                        }
                        // Lower/upper small wheels spin too
                        "lloswheels" | "rloswheels" | "lupswheels" | "rupswheels" => {
                            if is_moving {
                                tf.rotation = Quat::from_rotation_x(time * 5.0);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Recurse to find turret/wheels deeper in hierarchy
            animate_vehicle_pieces(child, children_query, name_query, transform_query, turret_yaw, is_moving, time);
        }
    }
}

// ============================================================
// Building Animations
// ============================================================

pub fn building_animation_system(
    buildings: Query<(Entity, &Building), With<Building>>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
    mut transform_query: Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    time: Res<Time>,
) {
    let t = time.elapsed_secs();

    for (entity, building) in &buildings {
        if !building.built {
            continue;
        }

        match building.building_type {
            BuildingType::MetalExtractor => {
                // arms1 and arms2 counter-rotate (BAR: spin around y-axis)
                animate_building_pieces(entity, &children_query, &name_query, &mut transform_query, |name, tf| {
                    match name {
                        "arms1" => { tf.rotation = Quat::from_rotation_y(t * 2.0); }
                        "arms2" => { tf.rotation = Quat::from_rotation_y(-t * 2.0); }
                        _ => {}
                    }
                });
            }
            BuildingType::RadarTower => {
                // dish spins around z-axis, turret around y-axis (BAR: dish z@180°/s, turret y@15°/s)
                animate_building_pieces(entity, &children_query, &name_query, &mut transform_query, |name, tf| {
                    match name {
                        "dish" => { tf.rotation = Quat::from_rotation_z(t * 3.14); }
                        "turret" => { tf.rotation = Quat::from_rotation_y(t * 0.26); }
                        _ => {}
                    }
                });
            }
            BuildingType::Factory => {
                // cagelight spins (BAR: cagelight_emit spins y@200°/s)
                animate_building_pieces(entity, &children_query, &name_query, &mut transform_query, |name, tf| {
                    match name {
                        "cagelight" | "cagelight_emit" => {
                            tf.rotation = Quat::from_rotation_y(t * 3.5);
                        }
                        _ => {}
                    }
                });
            }
            BuildingType::SolarCollector => {
                // Solar dishes don't animate in BAR, but we can add a subtle tilt
                // following the sun for visual interest (optional)
            }
            _ => {}
        }
    }
}

fn animate_building_pieces(
    entity: Entity,
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
    transform_query: &mut Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    visitor: impl Fn(&str, &mut Transform) + Copy,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            if let Ok(name) = name_query.get(child) {
                if let Ok(mut tf) = transform_query.get_mut(child) {
                    visitor(name.as_str(), &mut tf);
                }
            }
            animate_building_pieces(child, children_query, name_query, transform_query, visitor);
        }
    }
}

// ============================================================
// LLT Turret Aiming (separate because it needs enemy queries)
// ============================================================

pub fn llt_turret_animation_system(
    llts: Query<(Entity, &Transform, &Unit, &TeamOwned), (With<LightLaserTower>, With<Building>)>,
    all_units: Query<(&Transform, &TeamOwned), (With<Unit>, Without<Building>)>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
    mut transform_query: Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    time: Res<Time>,
) {
    let t = time.elapsed_secs();
    for (entity, llt_tf, unit, llt_team) in &llts {
        let llt_pos = game_xy(&llt_tf.translation);

        // Find nearest enemy
        let mut nearest_dir = Vec2::ZERO;
        let mut nearest_dist = f32::MAX;

        for (enemy_tf, enemy_team) in &all_units {
            if enemy_team.0 == llt_team.0 {
                continue;
            }
            let enemy_pos = game_xy(&enemy_tf.translation);
            let dist = llt_pos.distance(enemy_pos);
            if dist <= unit.attack_range && dist < nearest_dist {
                nearest_dist = dist;
                nearest_dir = (enemy_pos - llt_pos).normalize_or_zero();
            }
        }

        // Rotate turret and sleeve
        let turret_yaw = if nearest_dir.length_squared() > 0.01 {
            f32::atan2(nearest_dir.x, -nearest_dir.y)
        } else {
            // Idle scan
            (t * 0.3).sin() * 1.0
        };

        animate_llt_pieces(entity, &children_query, &name_query, &mut transform_query, turret_yaw);
    }
}

fn animate_llt_pieces(
    entity: Entity,
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
    transform_query: &mut Query<&mut Transform, (Without<Commander>, Without<Unit>)>,
    turret_yaw: f32,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            if let Ok(name) = name_query.get(child) {
                if let Ok(mut tf) = transform_query.get_mut(child) {
                    match name.as_str() {
                        "turret" => {
                            tf.rotation = Quat::from_rotation_y(turret_yaw);
                        }
                        "sleeve" => {
                            // Slight downward pitch toward ground targets
                            tf.rotation = Quat::from_rotation_x(deg(-10.0));
                        }
                        _ => {}
                    }
                }
            }
            animate_llt_pieces(child, children_query, name_query, transform_query, turret_yaw);
        }
    }
}

// ============================================================
// Unit Facing (non-commander, non-biped units face move/attack dir)
// ============================================================

pub fn unit_facing_system(
    mut units: Query<
        (&mut Transform, Option<&MoveTarget>, Option<&AttackTarget>),
        (With<Unit>, Without<Commander>, Without<Building>, Without<Artillery>),
    >,
    target_transforms: Query<&Transform, Or<(With<Commander>, With<Building>)>>,
    global_transforms: Query<&GlobalTransform>,
) {
    for (mut tf, move_target, attack_target) in &mut units {
        let pos = game_xy(&tf.translation);

        let face_dir = if let Some(MoveTarget(target)) = move_target {
            (*target - pos).normalize_or_zero()
        } else if let Some(AttackTarget(target_entity)) = attack_target {
            if let Ok(target_tf) = target_transforms.get(*target_entity) {
                (game_xy(&target_tf.translation) - pos).normalize_or_zero()
            } else if let Ok(gtf) = global_transforms.get(*target_entity) {
                let target_pos = gtf.translation();
                (game_xy(&target_pos) - pos).normalize_or_zero()
            } else {
                Vec2::ZERO
            }
        } else {
            Vec2::ZERO
        };

        if face_dir.length_squared() > 0.01 {
            let facing_angle = f32::atan2(face_dir.x, -face_dir.y);
            tf.rotation = Quat::from_rotation_y(facing_angle);
        }
    }
}

/// Facing system for artillery biped units
pub fn artillery_facing_system(
    mut units: Query<
        (&mut Transform, Option<&MoveTarget>, Option<&AttackTarget>),
        (With<Artillery>, Without<Commander>, Without<Building>),
    >,
    target_transforms: Query<&Transform, Or<(With<Commander>, With<Building>)>>,
    global_transforms: Query<&GlobalTransform>,
) {
    for (mut tf, move_target, attack_target) in &mut units {
        let pos = game_xy(&tf.translation);

        let face_dir = if let Some(MoveTarget(target)) = move_target {
            (*target - pos).normalize_or_zero()
        } else if let Some(AttackTarget(target_entity)) = attack_target {
            if let Ok(target_tf) = target_transforms.get(*target_entity) {
                (game_xy(&target_tf.translation) - pos).normalize_or_zero()
            } else if let Ok(gtf) = global_transforms.get(*target_entity) {
                let target_pos = gtf.translation();
                (game_xy(&target_pos) - pos).normalize_or_zero()
            } else {
                Vec2::ZERO
            }
        } else {
            Vec2::ZERO
        };

        if face_dir.length_squared() > 0.01 {
            let facing_angle = f32::atan2(face_dir.x, -face_dir.y);
            tf.rotation = Quat::from_rotation_y(facing_angle);
        }
    }
}
