mod networking;
mod setup;
mod spawning;
mod systems;
mod types;

use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport};
use bevy::render::RenderApp;
use bevy::window::WindowResolution;

use networking::*;
use setup::*;
use systems::*;
use types::*;

fn main() {
    let net_role = parse_cli_args();

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

    // 30Hz fixed timestep for deterministic lockstep simulation
    app.insert_resource(Time::<Fixed>::from_hz(30.0));

    // Set local player based on role
    let local_player = match &net_role {
        NetRole::Singleplayer | NetRole::Host { .. } => LocalPlayer { id: 0 },
        NetRole::Client { .. } => LocalPlayer { id: 1 },
    };

    app
        .init_resource::<AllTeamResources>()
        .insert_resource(local_player)
        .init_resource::<NextStableId>()
        .init_resource::<StableIdMap>()
        .init_resource::<BuildMode>()
        .init_resource::<DragSelect>()
        .init_resource::<CursorWorldPos>()
        .init_resource::<GameOver>()
        .init_resource::<DGunMode>()
        .init_resource::<LocalCommands>()
        .init_resource::<CommandBuffer>()
        .insert_resource(net_role.clone())
        .add_systems(Startup, (setup_camera, setup_map, setup_hud));

    // Input systems — run at render rate (Update)
    app.add_systems(
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
        )
            .chain(),
    );

    // Network sync or singleplayer command flush (runs in Update, before FixedUpdate)
    match &net_role {
        NetRole::Singleplayer => {
            app.add_systems(Update, singleplayer_command_flush);
        }
        NetRole::Host { port } => {
            setup_host_networking(&mut app, *port);
            app.add_systems(Update, (
                host_server_events,
                host_network_sync,
                lockstep_gate_system,
                desync_check_system,
                desync_receive_system,
            ));
        }
        NetRole::Client { addr } => {
            setup_client_networking(&mut app, *addr);
            app.add_systems(Update, (
                client_network_sync,
                lockstep_gate_system,
                desync_check_system,
                desync_receive_system,
            ));
        }
    }

    // Game simulation — run at fixed 30Hz (FixedUpdate)
    // apply_commands_system runs first, then simulation
    app.add_systems(
        FixedUpdate,
        (
            apply_commands_system,
            unit_movement,
            building_construction,
            terrain_follow_system,
        )
            .chain(),
    );
    app.add_systems(
        FixedUpdate,
        (
            combat_system,
            projectile_system,
            resource_production,
            factory_production,
            reclaim_system,
            wreckage_decay_system,
            unit_collision_system,
            building_collision_system,
            win_lose_check,
        ),
    );

    // Visual systems — run at render rate (Update)
    app.add_systems(
        Update,
        (
            fog_of_war_system,
            fog_overlay_system,
            nano_particle_system,
            death_explosion_system,
            health_bar_system,
            selection_indicator_system,
            build_ghost_system,
            hud_system,
            minimap_system,
            net_status_system,
        ),
    );

    // Animation systems — run at render rate (Update)
    app.add_systems(
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
    );

    app.run();
}
