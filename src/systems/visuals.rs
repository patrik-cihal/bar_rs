use bevy::prelude::*;

use crate::types::*;

// --- Visual Effects ---

pub fn death_explosion_system(
    mut commands: Commands,
    mut explosions: Query<(Entity, &mut DeathExplosion, &mut Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (entity, mut explosion, mut tf) in &mut explosions {
        explosion.timer += dt;
        let progress = explosion.timer / 0.5;

        if progress >= 1.0 {
            commands.entity(entity).despawn();
        } else {
            let scale = explosion.max_radius * 2.0 / 10.0 * progress;
            tf.scale = Vec3::splat(scale);
        }
    }
}

pub fn health_bar_system(
    units: Query<(&Unit, &Children)>,
    bg_query: Query<&Children, With<HealthBarBg>>,
    mut fill_query: Query<(&mut Transform, &MeshMaterial3d<StandardMaterial>), With<HealthBarFill>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
) {
    for (unit, children) in &units {
        let ratio = (unit.hp / unit.max_hp).clamp(0.0, 1.0);

        for child in children.iter() {
            if let Ok(bg_children) = bg_query.get(child) {
                for bg_child in bg_children.iter() {
                    if let Ok((mut fill_tf, mat_handle)) = fill_query.get_mut(bg_child) {
                        // Scale X to represent health fill
                        fill_tf.scale.x = ratio;
                        fill_tf.translation.x = (ratio - 1.0) * 15.0; // half-width offset

                        let color = if ratio > 0.5 {
                            Color::srgb(0.1, 0.9, 0.1)
                        } else if ratio > 0.25 {
                            Color::srgb(0.9, 0.9, 0.1)
                        } else {
                            Color::srgb(0.9, 0.1, 0.1)
                        };
                        if let Some(mat) = std_materials.get_mut(mat_handle) {
                            mat.base_color = color;
                        }
                    }
                }
            }
        }
    }
}

pub fn selection_indicator_system(
    selected_units: Query<&Transform, With<Selected>>,
    mut gizmos: Gizmos,
) {
    for transform in &selected_units {
        let pos = transform.translation;
        // Draw circle on ground plane (Y = slight offset above ground)
        gizmos.circle(
            Isometry3d::new(
                Vec3::new(pos.x, 0.2, pos.z),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            25.0,
            Color::srgb(0.0, 1.0, 0.0),
        );
    }
}

pub fn build_ghost_system(
    build_mode: Res<BuildMode>,
    cursor_pos: Res<CursorWorldPos>,
    terrain: Res<TerrainHeightmap>,
    metal_spots: Query<&Transform, With<MetalSpot>>,
    buildings: Query<(&Transform, &Building), Without<BuildGhost>>,
    mut ghost_q: Query<(&mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>), (With<BuildGhost>, Without<MetalSpot>, Without<Building>)>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((mut tf, mut vis, mat_handle)) = ghost_q.single_mut() else {
        return;
    };

    if !build_mode.active {
        *vis = Visibility::Hidden;
        return;
    }

    *vis = Visibility::Visible;
    let mut pos = cursor_pos.0;

    let Some(btype) = build_mode.building_type else {
        *vis = Visibility::Hidden;
        return;
    };

    if btype == BuildingType::MetalExtractor {
        for spot_tf in &metal_spots {
            let spot_pos = game_xy(&spot_tf.translation);
            if spot_pos.distance(pos) < EXTRACTOR_SNAP_RANGE {
                pos = spot_pos;
                break;
            }
        }
    }

    let bs = btype.stats();
    let size = Vec2::new(bs.size.0, bs.size.1);

    // Snap to build grid
    pos = snap_to_build_grid(pos, bs.size);

    // Check if placement is valid: terrain flatness + no building overlap
    let flat_enough = terrain.is_flat_enough(pos.x, pos.y, size.x, size.y);
    let overlaps_building = buildings.iter().any(|(b_tf, b)| {
        let b_pos = game_xy(&b_tf.translation);
        let b_stats = b.building_type.stats();
        let b_half = Vec2::new(b_stats.size.0 * 0.5, b_stats.size.1 * 0.5);
        let half = size * 0.5;
        // AABB overlap check
        (pos.x - half.x) < (b_pos.x + b_half.x)
            && (pos.x + half.x) > (b_pos.x - b_half.x)
            && (pos.y - half.y) < (b_pos.y + b_half.y)
            && (pos.y + half.y) > (b_pos.y - b_half.y)
    });
    // Extractors can only be placed on metal spots
    let on_metal_spot = if btype == BuildingType::MetalExtractor {
        metal_spots.iter().any(|spot_tf| {
            game_xy(&spot_tf.translation).distance(pos) < EXTRACTOR_SNAP_RANGE
        })
    } else {
        true
    };
    let can_place = flat_enough && !overlaps_building && on_metal_spot;

    // Update ghost color: green = valid, red = invalid
    if let Some(mat) = std_materials.get_mut(mat_handle) {
        mat.base_color = if can_place {
            Color::srgba(0.5, 1.0, 0.5, 0.4)
        } else {
            Color::srgba(1.0, 0.3, 0.3, 0.4)
        };
    }

    let world_pos = game_pos(pos.x, pos.y, terrain.height_at(pos.x, pos.y) + 0.5);
    tf.translation = world_pos;
    tf.scale = Vec3::new(size.x / 40.0, 1.0, size.y / 40.0); // Scale relative to base ghost mesh size
}

// --- HUD ---

pub fn hud_system(
    resources: Res<GameResources>,
    build_mode: Res<BuildMode>,
    dgun_mode: Res<DGunMode>,
    mut metal_text: Query<&mut Text, (With<HudMetal>, Without<HudEnergy>, Without<HudBuildHint>, Without<HudFactoryQueue>)>,
    mut energy_text: Query<&mut Text, (With<HudEnergy>, Without<HudMetal>, Without<HudBuildHint>, Without<HudFactoryQueue>)>,
    mut hint_text: Query<&mut Text, (With<HudBuildHint>, Without<HudMetal>, Without<HudEnergy>, Without<HudFactoryQueue>)>,
    mut queue_text: Query<&mut Text, (With<HudFactoryQueue>, Without<HudMetal>, Without<HudEnergy>, Without<HudBuildHint>)>,
    selected_commanders: Query<(), (With<Selected>, With<Commander>, With<PlayerOwned>)>,
    selected_factories: Query<&Factory, (With<Selected>, With<PlayerOwned>)>,
    selected_non_cmd: Query<(), (With<Selected>, With<PlayerOwned>, With<Unit>, Without<Commander>, Without<Factory>)>,
) {
    for mut text in &mut metal_text {
        **text = format!(
            "Metal: {:.0} (+{:.0}/s)",
            resources.metal, resources.metal_income
        );
    }
    for mut text in &mut energy_text {
        **text = format!(
            "Energy: {:.0} (+{:.0}/s)",
            resources.energy, resources.energy_income
        );
    }
    for mut text in &mut hint_text {
        if dgun_mode.0 {
            **text = "D-GUN MODE — Click to fire (500 energy) | Esc to cancel".to_string();
        } else if build_mode.active {
            let name = match build_mode.building_type {
                Some(BuildingType::MetalExtractor) => "Metal Extractor",
                Some(BuildingType::SolarCollector) => "Solar Collector",
                Some(BuildingType::Factory) => "Factory",
                Some(BuildingType::LLT) => "LLT",
                Some(BuildingType::Wall) => "Wall",
                Some(BuildingType::RadarTower) => "Radar Tower",
                None => "???",
            };
            **text = format!("Placing: {} - Click to place, Esc to cancel", name);
        } else if !selected_commanders.is_empty() {
            **text = "[1]Ext [2]Solar [3]Fac [4]LLT [5]Wall [6]Radar [G]D-Gun | RClick: Move/Atk".to_string();
        } else if !selected_factories.is_empty() {
            **text = "[Q]Scout [W]Raider [E]Tank [R]Assault [T]Artillery | Shift+key: x5".to_string();
        } else if !selected_non_cmd.is_empty() {
            **text = "Right-click: Move/Attack".to_string();
        } else {
            **text = String::new();
        }
    }

    for mut text in &mut queue_text {
        let mut queue_str = String::new();
        for factory in &selected_factories {
            if !factory.queue.is_empty() {
                queue_str = format!(
                    "Queue: {}",
                    factory
                        .queue
                        .iter()
                        .map(|u| u.stats().name)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                break;
            }
        }
        **text = queue_str;
    }
}

// --- Minimap ---

pub fn minimap_system(
    _gizmos: Gizmos,
    player_units: Query<&Transform, (With<Unit>, With<PlayerOwned>)>,
    enemy_units: Query<(&Transform, &Visibility), (With<Unit>, With<EnemyOwned>)>,
    player_buildings: Query<&Transform, (With<Building>, With<PlayerOwned>, Without<Unit>)>,
    metal_spots: Query<&Transform, (With<MetalSpot>, Without<Unit>, Without<Building>)>,
    mouse: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window>,
    mut camera_q: Query<&mut Transform, (With<Camera3d>, Without<Unit>, Without<Building>, Without<MetalSpot>)>,
) {
    let Ok(window) = window_q.single() else { return };
    let window_h = window.resolution.height();

    let minimap_screen_left = 10.0;
    let minimap_screen_bottom = 10.0;
    let minimap_screen_top = minimap_screen_bottom + MINIMAP_SIZE;

    if mouse.just_pressed(MouseButton::Left) {
        if let Some(cursor) = window.cursor_position() {
            let cursor_from_bottom = window_h - cursor.y;
            if cursor.x >= minimap_screen_left
                && cursor.x <= minimap_screen_left + MINIMAP_SIZE
                && cursor_from_bottom >= minimap_screen_bottom
                && cursor_from_bottom <= minimap_screen_top
            {
                let rel_x = (cursor.x - minimap_screen_left) / MINIMAP_SIZE;
                let rel_y = (cursor_from_bottom - minimap_screen_bottom) / MINIMAP_SIZE;
                let world_x = rel_x * MAP_SIZE;
                let world_y = rel_y * MAP_SIZE;

                if let Ok(mut cam_tf) = camera_q.single_mut() {
                    // Move camera to look at this world position
                    // Keep the camera's relative offset (height + angle offset)
                    let pitch = 55.0_f32.to_radians();
                    let cam_dist = 500.0;
                    cam_tf.translation.x = world_x;
                    cam_tf.translation.y = cam_dist * pitch.sin();
                    cam_tf.translation.z = -world_y + cam_dist * pitch.cos();
                }
            }
        }
    }

    let _ = (player_units, enemy_units, player_buildings, metal_spots, _gizmos);
}

// --- Win/Lose ---

pub fn win_lose_check(
    mut commands: Commands,
    mut game_over: ResMut<GameOver>,
    player_commanders: Query<Entity, (With<Commander>, With<PlayerOwned>)>,
    enemy_units: Query<Entity, (With<Unit>, With<EnemyOwned>)>,
    existing_go_text: Query<Entity, With<GameOverText>>,
) {
    if game_over.0.is_some() {
        return;
    }

    if player_commanders.is_empty() {
        game_over.0 = Some("DEFEAT - Your commander has been destroyed!".to_string());
    } else if enemy_units.is_empty() {
        game_over.0 = Some("VICTORY - All enemy forces destroyed!".to_string());
    }

    if let Some(ref msg) = game_over.0 {
        if existing_go_text.is_empty() {
            commands.spawn((
                Text::new(msg.clone()),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(40.0),
                    left: Val::Percent(20.0),
                    ..default()
                },
                GameOverText,
            ));
        }
    }
}
