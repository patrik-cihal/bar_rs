use bevy::prelude::*;
use bevy::light::{CascadeShadowConfigBuilder, GlobalAmbientLight};
use bevy::mesh::{Indices, VertexAttributeValues, PrimitiveTopology};
use bevy::post_process::bloom::Bloom;
use bevy::render::view::NoIndirectDrawing;
use std::collections::HashMap;

use crate::networking::NetRole;
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
        Bloom { intensity: 0.15, ..Bloom::NATURAL },
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
            -0.7, // moderate angle down (less steep = longer shadows)
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

    // Ambient light so shadowed/backlit terrain isn't pitch black
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.6, 0.65, 0.8),
        brightness: 200.0,
        ..default()
    });
}

pub fn setup_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut next_stable_id: ResMut<NextStableId>,
    mut stable_id_map: ResMut<StableIdMap>,
    net_role: Res<NetRole>,
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

    // Generate terrain heightmap
    let terrain = TerrainHeightmap::generate();

    // Debug: print height range
    let min_h = terrain.heights.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_h = terrain.heights.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    info!("Terrain heights: min={:.2}, max={:.2}", min_h, max_h);

    // Build terrain mesh from heightmap
    let terrain_mesh = build_terrain_mesh(&terrain);
    commands.spawn((
        Mesh3d(meshes.add(terrain_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.95,
            metallic: 0.0,
            ..default()
        })),
        Transform::IDENTITY,
    ));

    // Fog of war overlay mesh — sits just above terrain, vertex alpha driven by sight
    let fog_mesh = build_fog_mesh(&terrain);
    commands.spawn((
        Mesh3d(meshes.add(fog_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE, // vertex colors provide the actual color+alpha
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::IDENTITY,
        FogOverlay,
    ));

    // Metal spots (from shared const, grid-aligned)
    for &(mx, my) in &METAL_SPOT_POSITIONS {
        let pos = &Vec2::new(mx, my);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(20.0, 2.0, 20.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.7, 0.9),
                emissive: LinearRgba::new(0.8, 1.2, 2.0, 1.0),
                ..default()
            })),
            Transform::from_translation(game_pos(pos.x, pos.y, terrain.height_at(pos.x, pos.y)))
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
            Transform::from_translation(game_pos(pos.x, pos.y, terrain.height_at(pos.x, pos.y) + 0.05)),
            MapFeature { metal_value: metal },
        ));
    }

    // Player commander (bottom-left)
    spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(200.0, 200.0),
        0, // team 0 = blue
        None,
        &model_library,
        &mut next_stable_id,
        &mut stable_id_map,
    );

    // Enemy commander (top-right)
    spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec2::new(1800.0, 1800.0),
        1, // team 1 = red
        None,
        &model_library,
        &mut next_stable_id,
        &mut stable_id_map,
    );

    // In singleplayer, spawn enemy tanks and buildings for AI opponent
    // In multiplayer, player 2 starts with just a commander (symmetric)
    if matches!(*net_role, NetRole::Singleplayer) {
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
                1,
                Some(UnitType::Tank),
                &model_library,
                &mut next_stable_id,
                &mut stable_id_map,
            );
        }

        // Enemy buildings (snapped to build grid, extractor on a metal spot)
        let enemy_ext_pos = snap_to_build_grid(Vec2::new(1800.0, 1800.0), BuildingType::MetalExtractor.stats().size);
        spawn_building_entity(
            &mut commands,
            &mut meshes,
            &mut materials,
            enemy_ext_pos,
            BuildingType::MetalExtractor,
            1,
            true,
            &model_library,
            &mut next_stable_id,
            &mut stable_id_map,
        );
        let enemy_solar_pos = snap_to_build_grid(Vec2::new(1760.0, 1696.0), BuildingType::SolarCollector.stats().size);
        spawn_building_entity(
            &mut commands,
            &mut meshes,
            &mut materials,
            enemy_solar_pos,
            BuildingType::SolarCollector,
            1,
            true,
            &model_library,
            &mut next_stable_id,
            &mut stable_id_map,
        );
        let enemy_fac_pos = snap_to_build_grid(Vec2::new(1600.0, 1696.0), BuildingType::Factory.stats().size);
        spawn_building_entity(
            &mut commands,
            &mut meshes,
            &mut materials,
            enemy_fac_pos,
            BuildingType::Factory,
            1,
            true,
            &model_library,
            &mut next_stable_id,
            &mut stable_id_map,
        );
    }

    let nav_grid = NavGrid::new(&terrain);
    commands.insert_resource(model_library);
    commands.insert_resource(terrain);
    commands.insert_resource(nav_grid);

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

    // Network status (top-right)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn((
            Text::new(""),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.5, 1.0, 0.5)),
            HudNetStatus,
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

fn build_terrain_mesh(terrain: &TerrainHeightmap) -> Mesh {
    let gs = TERRAIN_GRID_SIZE;
    let num_verts = gs * gs;
    let mut positions = Vec::with_capacity(num_verts);
    let mut normals = Vec::with_capacity(num_verts);
    let mut colors = Vec::with_capacity(num_verts);
    let mut uvs = Vec::with_capacity(num_verts);

    // Generate vertices
    for gy in 0..gs {
        for gx in 0..gs {
            let wx = gx as f32 * terrain.cell_size;
            let wy = gy as f32 * terrain.cell_size;
            let h = terrain.heights[gy * gs + gx];

            // World coords: game (x,y) -> world (x, height, -y)
            positions.push([wx, h, -wy]);
            uvs.push([gx as f32 / (gs - 1) as f32, gy as f32 / (gs - 1) as f32]);

            // Color based on height: green lowlands -> brown hills -> grey peaks
            // Vertex colors replace base_color in PBR shader (linear space)
            // Use Color::srgb().to_linear() equivalent by providing sRGB values directly
            // since Bevy's vertex colors go through sRGB->linear in the pipeline
            let t = (h / TERRAIN_MAX_HEIGHT).clamp(0.0, 1.0);
            let color = if t < 0.3 {
                // Dark green to light green
                let s = t / 0.3;
                Color::srgb(0.30 + s * 0.15, 0.50 + s * 0.15, 0.20 + s * 0.08)
            } else if t < 0.7 {
                // Green to brown
                let s = (t - 0.3) / 0.4;
                Color::srgb(0.45 + s * 0.25, 0.65 - s * 0.20, 0.28 - s * 0.05)
            } else {
                // Brown to grey rock
                let s = (t - 0.7) / 0.3;
                Color::srgb(0.70 + s * 0.10, 0.45 + s * 0.20, 0.23 + s * 0.25)
            };
            let lin = color.to_linear();
            colors.push([lin.red, lin.green, lin.blue, 1.0]);

            // Placeholder normal (will compute below)
            normals.push([0.0, 1.0, 0.0]);
        }
    }

    // Compute normals from neighboring heights
    for gy in 0..gs {
        for gx in 0..gs {
            let idx = gy * gs + gx;
            let h_l = if gx > 0 { terrain.heights[idx - 1] } else { terrain.heights[idx] };
            let h_r = if gx < gs - 1 { terrain.heights[idx + 1] } else { terrain.heights[idx] };
            let h_d = if gy > 0 { terrain.heights[idx - gs] } else { terrain.heights[idx] };
            let h_u = if gy < gs - 1 { terrain.heights[idx + gs] } else { terrain.heights[idx] };

            let dx = (h_r - h_l) / (2.0 * terrain.cell_size);
            let dy = (h_u - h_d) / (2.0 * terrain.cell_size);
            let n = Vec3::new(-dx, 1.0, dy).normalize();
            normals[idx] = [n.x, n.y, n.z];
        }
    }

    // Generate triangle indices
    let num_quads = (gs - 1) * (gs - 1);
    let mut indices = Vec::with_capacity(num_quads * 6);
    for gy in 0..(gs - 1) {
        for gx in 0..(gs - 1) {
            let i = (gy * gs + gx) as u32;
            let row = gs as u32;
            // Two triangles per quad (CCW winding for upward-facing normals)
            indices.push(i);
            indices.push(i + 1);
            indices.push(i + row);

            indices.push(i + 1);
            indices.push(i + row + 1);
            indices.push(i + row);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, VertexAttributeValues::Float32x4(colors));
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Build the fog of war overlay mesh. Same grid as terrain but offset slightly
/// above. Starts fully dark (alpha=0.7); the fog_overlay_system clears areas
/// around player units each frame.
pub fn build_fog_mesh(terrain: &TerrainHeightmap) -> Mesh {
    let gs = FOG_GRID_SIZE;
    let cell_size = MAP_SIZE / (gs - 1) as f32;
    let num_verts = gs * gs;
    let mut positions = Vec::with_capacity(num_verts);
    let mut normals = Vec::with_capacity(num_verts);
    let mut colors = Vec::with_capacity(num_verts);
    let mut uvs = Vec::with_capacity(num_verts);

    for gy in 0..gs {
        for gx in 0..gs {
            let wx = gx as f32 * cell_size;
            let wy = gy as f32 * cell_size;
            let h = terrain.height_at(wx, wy) + 0.5; // slightly above terrain
            positions.push([wx, h, -wy]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([gx as f32 / (gs - 1) as f32, gy as f32 / (gs - 1) as f32]);
            // Dark fog, starts opaque
            colors.push([0.0_f32, 0.0, 0.0, 0.7]);
        }
    }

    let num_quads = (gs - 1) * (gs - 1);
    let mut indices = Vec::with_capacity(num_quads * 6);
    for gy in 0..(gs - 1) {
        for gx in 0..(gs - 1) {
            let i = (gy * gs + gx) as u32;
            let row = gs as u32;
            indices.push(i);
            indices.push(i + 1);
            indices.push(i + row);
            indices.push(i + 1);
            indices.push(i + row + 1);
            indices.push(i + row);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, VertexAttributeValues::Float32x4(colors));
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
