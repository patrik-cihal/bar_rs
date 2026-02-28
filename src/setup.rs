use bevy::prelude::*;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::render::view::NoIndirectDrawing;
use std::collections::HashMap;

use crate::spawning::*;
use crate::types::*;

pub fn setup_camera(mut commands: Commands) {
    // 3D orthographic camera at ~55° from horizontal (RTS-style angled view)
    let target = Vec3::new(200.0, 0.0, -200.0);
    let pitch = 55.0_f32.to_radians();
    let cam_dist = 500.0;
    let eye = target + Vec3::new(0.0, cam_dist * pitch.sin(), cam_dist * pitch.cos());

    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(eye).looking_at(target, Vec3::Y),
        NoIndirectDrawing,
    ));

    // Directional light (sun) — angled from upper-left, with shadows
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -1.0, // steep angle down
            0.5,  // slightly from the side
            0.0,
        )),
        CascadeShadowConfigBuilder {
            maximum_distance: 3000.0,
            num_cascades: 4,
            minimum_distance: 1.0,
            ..default()
        }
        .build(),
    ));

    // Fill light from opposite side (no shadows)
    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -0.8,
            -1.5,
            0.0,
        )),
    ));
}

pub fn setup_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Load all unit and building models (blue + red variants)
    let mut models = HashMap::new();
    for name in ["armcom", "armpeep", "armflash", "armstump", "armbull", "armham",
                 "armmex", "armsolar", "armlab", "armllt", "armdrag", "armrad"] {
        for color in ["blue", "red"] {
            let key = format!("{}_{}", name, color);
            let path = format!("models/{}_{}.glb#Scene0", name, color);
            models.insert(key, asset_server.load(&path));
        }
    }
    let model_library = ModelLibrary { models };

    // Map background — large flat box on ground
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(MAP_SIZE, 0.1, MAP_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.35, 0.2),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_translation(game_pos(MAP_SIZE / 2.0, MAP_SIZE / 2.0, -0.05)),
    ));

    // Metal spots
    let metal_spots = vec![
        Vec2::new(300.0, 300.0),
        Vec2::new(500.0, 200.0),
        Vec2::new(200.0, 600.0),
        Vec2::new(700.0, 400.0),
        Vec2::new(1000.0, 1000.0),
        Vec2::new(1300.0, 1600.0),
        Vec2::new(1500.0, 1400.0),
        Vec2::new(1700.0, 1800.0),
        Vec2::new(1800.0, 1500.0),
        Vec2::new(1600.0, 1700.0),
    ];

    for pos in &metal_spots {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(20.0, 2.0, 20.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.6, 0.6),
                unlit: false,
                ..default()
            })),
            Transform::from_translation(game_pos(pos.x, pos.y, 0.0))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)),
            MetalSpot,
        ));
    }

    // Scatter rocks and trees (reclaimable map features)
    let features = vec![
        (Vec2::new(400.0, 400.0), 8.0, Color::srgb(0.4, 0.4, 0.35)),
        (Vec2::new(600.0, 300.0), 6.0, Color::srgb(0.3, 0.5, 0.2)),
        (Vec2::new(150.0, 450.0), 7.0, Color::srgb(0.4, 0.4, 0.35)),
        (Vec2::new(800.0, 600.0), 10.0, Color::srgb(0.3, 0.5, 0.2)),
        (Vec2::new(350.0, 700.0), 5.0, Color::srgb(0.4, 0.4, 0.35)),
        (Vec2::new(900.0, 300.0), 8.0, Color::srgb(0.3, 0.5, 0.2)),
        (Vec2::new(1100.0, 800.0), 7.0, Color::srgb(0.4, 0.4, 0.35)),
        (Vec2::new(1200.0, 1200.0), 9.0, Color::srgb(0.3, 0.5, 0.2)),
        (Vec2::new(1400.0, 1000.0), 6.0, Color::srgb(0.4, 0.4, 0.35)),
        (Vec2::new(1600.0, 1300.0), 8.0, Color::srgb(0.3, 0.5, 0.2)),
        (Vec2::new(1100.0, 500.0), 7.0, Color::srgb(0.4, 0.4, 0.35)),
        (Vec2::new(500.0, 900.0), 6.0, Color::srgb(0.3, 0.5, 0.2)),
    ];

    for (pos, metal, color) in features {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(12.0, 6.0, 12.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                unlit: false,
                ..default()
            })),
            Transform::from_translation(game_pos(pos.x, pos.y, 0.05)),
            MapFeature { metal_value: metal },
        ));
    }

    // Player commander (bottom-left)
    spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(200.0, 200.0),
        true,
        None,
        &model_library,
    );

    // Player buildings near start (for testing animations)
    spawn_building_entity(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(300.0, 300.0),
        BuildingType::MetalExtractor,
        true,
        true,
        &model_library,
    );
    spawn_building_entity(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(350.0, 200.0),
        BuildingType::SolarCollector,
        true,
        true,
        &model_library,
    );

    // Enemy commander (top-right)
    spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(1800.0, 1800.0),
        false,
        None,
        &model_library,
    );

    // Enemy tanks
    for offset in &[
        Vec2::new(-80.0, 0.0),
        Vec2::new(0.0, -80.0),
        Vec2::new(-80.0, -80.0),
        Vec2::new(-160.0, 0.0),
    ] {
        spawn_unit(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec2::new(1800.0, 1800.0) + *offset,
            false,
            Some(UnitType::Tank),
            &model_library,
        );
    }

    // Enemy buildings
    spawn_building_entity(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(1700.0, 1800.0),
        BuildingType::MetalExtractor,
        false,
        true,
        &model_library,
    );
    spawn_building_entity(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(1800.0, 1700.0),
        BuildingType::SolarCollector,
        false,
        true,
        &model_library,
    );
    spawn_building_entity(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(1600.0, 1700.0),
        BuildingType::Factory,
        false,
        true,
        &model_library,
    );

    commands.insert_resource(model_library);

    // Build ghost entity (invisible initially) — flat 3D quad
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(40.0, 2.0, 40.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 1.0, 0.5, 0.4),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_translation(game_pos(-9999.0, -9999.0, 0.5)),
        Visibility::Hidden,
        BuildGhost,
    ));

    // Selection box entity (invisible initially) — flat 3D quad
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 0.1, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 1.0, 0.0, 0.15),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_translation(game_pos(0.0, 0.0, 1.0)),
        Visibility::Hidden,
        SelectionBox,
    ));
}

pub fn setup_hud(mut commands: Commands) {
    // Root HUD container
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new("Metal: 1000 (+0/s)"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.9)),
                HudMetal,
            ));
            parent.spawn((
                Text::new("Energy: 1000 (+0/s)"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.3)),
                HudEnergy,
            ));
            parent.spawn((
                Text::new("[1] Extractor  [2] Solar  [3] Factory  [Esc] Cancel"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                HudBuildHint,
            ));
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.6)),
                HudFactoryQueue,
            ));
        });

    // Minimap frame (bottom-left)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            width: Val::Px(MINIMAP_SIZE),
            height: Val::Px(MINIMAP_SIZE),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        MinimapFrame,
    ));
}
