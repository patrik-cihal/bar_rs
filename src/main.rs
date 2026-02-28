mod setup;
mod spawning;
mod systems;
mod types;

use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport};
use bevy::render::RenderApp;
use bevy::window::WindowResolution;

use setup::*;
use systems::*;
use types::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Beyond All Reason - Bevy RTS".into(),
                resolution: WindowResolution::from([1280u32, 720]),
                ..default()
            }),
            ..default()
        }));

    // Disable GPU preprocessing to fix flickering on Intel integrated GPUs
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.insert_resource(GpuPreprocessingSupport {
            max_supported_mode: GpuPreprocessingMode::None,
        });
    }

    app
        .init_resource::<GameResources>()
        .init_resource::<BuildMode>()
        .init_resource::<DragSelect>()
        .init_resource::<CursorWorldPos>()
        .init_resource::<GameOver>()
        .init_resource::<DGunMode>()
        .add_systems(Startup, (setup_camera, setup_map, setup_hud))
        .add_systems(
            Update,
            (
                update_cursor_world_pos,
                camera_movement,
                unit_selection,
                build_mode_input,
                building_placement,
                unit_commands,
                factory_queue_input,
                dgun_input,
                unit_movement,
                building_construction,
                terrain_follow_system,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                combat_system,
                projectile_system,
                resource_production,
                factory_production,
                reclaim_system,
                wreckage_decay_system,
                unit_collision_system,
                building_collision_system,
                fog_of_war_system,
                fog_overlay_system,
                nano_particle_system,
            ),
        )
        .add_systems(
            Update,
            (
                death_explosion_system,
                health_bar_system,
                selection_indicator_system,
                build_ghost_system,
                hud_system,
                minimap_system,
                win_lose_check,
            ),
        )
        .add_systems(
            Update,
            (
                commander_animation_system,
                biped_walk_animation_system,
                vehicle_animation_system,
                building_animation_system,
                llt_turret_animation_system,
                unit_facing_system,
                artillery_facing_system,
            ),
        )
        .run();
}
