use bevy::prelude::*;
use bevy::ecs::message::MessageReader;
use std::collections::HashMap;
use std::net::{UdpSocket, SocketAddr};
use std::time::SystemTime;

use bevy_renet2::prelude::*;
use bevy_renet2::netcode::*;

use crate::spawning::*;
use crate::types::*;

// --- Constants ---

const PROTOCOL_ID: u64 = 0xBA4_4057;
const INPUT_DELAY: u64 = 3; // 3 ticks (~100ms at 30Hz)

// --- Network Role ---

#[derive(Resource, Clone, Debug, PartialEq)]
pub enum NetRole {
    Singleplayer,
    Host { port: u16 },
    Client { addr: SocketAddr },
}

impl Default for NetRole {
    fn default() -> Self {
        Self::Singleplayer
    }
}

// --- Game Command Protocol ---

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum GameCommand {
    MoveUnits { unit_ids: Vec<u64>, target: (f32, f32) },
    AttackUnits { unit_ids: Vec<u64>, target_id: u64 },
    PlaceBuilding { building_type: u8, position: (f32, f32), commander_ids: Vec<u64> },
    QueueUnit { factory_id: u64, unit_type: u8 },
    DGun { commander_id: u64, target_pos: (f32, f32) },
    Reclaim { commander_id: u64, target_id: u64 },
}

/// Per-tick input from a player
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TickInput {
    pub tick: u64,
    pub player: u8,
    pub commands: Vec<GameCommand>,
}

/// Accumulates commands from local input each frame (before they're assigned to a tick)
#[derive(Resource, Default)]
pub struct LocalCommands {
    pub commands: Vec<GameCommand>,
}

/// Stores commands per tick for both players. Simulation only advances when commands
/// for the current tick are available.
#[derive(Resource)]
pub struct CommandBuffer {
    pub current_tick: u64,
    /// Commands for each tick, indexed by (tick, player)
    pub pending: HashMap<(u64, u8), Vec<GameCommand>>,
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self {
            current_tick: 0,
            pending: HashMap::new(),
        }
    }
}

impl CommandBuffer {
    /// In singleplayer, push local commands directly for the current tick
    pub fn push_singleplayer(&mut self, commands: Vec<GameCommand>) {
        let tick = self.current_tick;
        self.pending.entry((tick, 0)).or_default().extend(commands);
        // Enemy AI has no commands (handled by systems directly)
        self.pending.entry((tick, 1)).or_default();
    }

    /// Get commands for a specific tick and player, removing them from buffer
    pub fn take_commands(&mut self, tick: u64, player: u8) -> Vec<GameCommand> {
        self.pending.remove(&(tick, player)).unwrap_or_default()
    }

    /// Check if commands for both players are available for the given tick
    pub fn has_commands_for_tick(&self, tick: u64) -> bool {
        self.pending.contains_key(&(tick, 0)) && self.pending.contains_key(&(tick, 1))
    }
}

/// Helper to convert BuildingType to/from u8
impl BuildingType {
    pub fn to_u8(&self) -> u8 {
        match self {
            BuildingType::MetalExtractor => 0,
            BuildingType::SolarCollector => 1,
            BuildingType::Factory => 2,
            BuildingType::LLT => 3,
            BuildingType::Wall => 4,
            BuildingType::RadarTower => 5,
        }
    }

    pub fn from_u8(v: u8) -> Option<BuildingType> {
        match v {
            0 => Some(BuildingType::MetalExtractor),
            1 => Some(BuildingType::SolarCollector),
            2 => Some(BuildingType::Factory),
            3 => Some(BuildingType::LLT),
            4 => Some(BuildingType::Wall),
            5 => Some(BuildingType::RadarTower),
            _ => None,
        }
    }
}

/// Helper to convert UnitType to/from u8
impl UnitType {
    pub fn to_u8(&self) -> u8 {
        match self {
            UnitType::Scout => 0,
            UnitType::Raider => 1,
            UnitType::Tank => 2,
            UnitType::Assault => 3,
            UnitType::Artillery => 4,
        }
    }

    pub fn from_u8(v: u8) -> Option<UnitType> {
        match v {
            0 => Some(UnitType::Scout),
            1 => Some(UnitType::Raider),
            2 => Some(UnitType::Tank),
            3 => Some(UnitType::Assault),
            4 => Some(UnitType::Artillery),
            _ => None,
        }
    }
}

// --- CLI Parsing ---

pub fn parse_cli_args() -> NetRole {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                let port = args.get(i + 1)
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(12345);
                return NetRole::Host { port };
            }
            "--connect" => {
                let addr_str = args.get(i + 1).cloned().unwrap_or_else(|| "127.0.0.1:12345".to_string());
                let addr: SocketAddr = addr_str.parse().unwrap_or_else(|_| {
                    eprintln!("Invalid address: {}, using default", addr_str);
                    "127.0.0.1:12345".parse().unwrap()
                });
                return NetRole::Client { addr };
            }
            _ => {}
        }
        i += 1;
    }
    NetRole::Singleplayer
}

// --- Network Setup ---

pub fn setup_host_networking(app: &mut App, port: u16) {
    app.add_plugins(RenetServerPlugin);
    app.add_plugins(NetcodeServerPlugin);

    let server = RenetServer::new(ConnectionConfig::from_shared_channels(DefaultChannel::config()));
    app.insert_resource(server);

    let server_addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let socket = UdpSocket::bind(server_addr).expect("Failed to bind server socket");
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let server_config = ServerSetupConfig {
        current_time,
        max_clients: 1,
        protocol_id: PROTOCOL_ID,
        socket_addresses: vec![vec![server_addr]],
        authentication: ServerAuthentication::Unsecure,
    };
    let transport = NetcodeServerTransport::new(server_config, NativeSocket::new(socket).unwrap()).unwrap();
    app.insert_resource(transport);

    info!("Hosting on port {}", port);
}

pub fn setup_client_networking(app: &mut App, server_addr: SocketAddr) {
    app.add_plugins(RenetClientPlugin);
    app.add_plugins(NetcodeClientPlugin);

    let client = RenetClient::new(ConnectionConfig::from_shared_channels(DefaultChannel::config()), false);
    app.insert_resource(client);

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind client socket");
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id,
        user_data: None,
        protocol_id: PROTOCOL_ID,
        socket_id: 0,
    };
    let transport = NetcodeClientTransport::new(current_time, authentication, NativeSocket::new(socket).unwrap()).unwrap();
    app.insert_resource(transport);

    info!("Connecting to {}", server_addr);
}

// --- Serialization helpers ---

fn encode_tick_input(input: &TickInput) -> Vec<u8> {
    bincode::serde::encode_to_vec(input, bincode::config::standard()).unwrap()
}

fn decode_tick_input(data: &[u8]) -> Option<TickInput> {
    bincode::serde::decode_from_slice::<TickInput, _>(data, bincode::config::standard())
        .ok()
        .map(|(input, _)| input)
}

// --- Singleplayer Systems ---

/// Singleplayer: flush local commands into command buffer each frame
pub fn singleplayer_command_flush(
    mut local_commands: ResMut<LocalCommands>,
    mut command_buffer: ResMut<CommandBuffer>,
) {
    let cmds = std::mem::take(&mut local_commands.commands);
    command_buffer.push_singleplayer(cmds);
}

// --- Multiplayer Host Systems ---

/// Host: handle server events (connection/disconnection)
pub fn host_server_events(
    mut server_events: MessageReader<ServerEvent>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                info!("Client {} connected", client_id);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("Client {} disconnected: {:?}", client_id, reason);
            }
        }
    }
}

/// Host: send local commands and receive remote commands
pub fn host_network_sync(
    mut local_commands: ResMut<LocalCommands>,
    mut command_buffer: ResMut<CommandBuffer>,
    mut server: ResMut<RenetServer>,
    local_player: Res<LocalPlayer>,
) {
    let cmds = std::mem::take(&mut local_commands.commands);
    let local_tick = command_buffer.current_tick + INPUT_DELAY;

    let local_input = TickInput {
        tick: local_tick,
        player: local_player.id,
        commands: cmds,
    };

    // Store host's own commands
    command_buffer.pending
        .entry((local_input.tick, local_input.player))
        .or_default()
        .extend(local_input.commands.clone());

    // Send host's commands to remote client
    let encoded = encode_tick_input(&local_input);
    server.broadcast_message(DefaultChannel::ReliableOrdered, encoded);

    // Receive remote client's commands
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
            if let Some(remote_input) = decode_tick_input(&message) {
                // Store remote commands
                command_buffer.pending
                    .entry((remote_input.tick, remote_input.player))
                    .or_default()
                    .extend(remote_input.commands.clone());

                // Relay remote commands back to all clients
                let relay = encode_tick_input(&remote_input);
                server.broadcast_message(DefaultChannel::ReliableOrdered, relay);
            }
        }
    }

    // Ensure host's tick entry exists
    command_buffer.pending.entry((local_tick, 0)).or_default();
}

/// Client: send local commands and receive commands from host
pub fn client_network_sync(
    mut local_commands: ResMut<LocalCommands>,
    mut command_buffer: ResMut<CommandBuffer>,
    mut client: ResMut<RenetClient>,
    local_player: Res<LocalPlayer>,
) {
    let cmds = std::mem::take(&mut local_commands.commands);
    let local_tick = command_buffer.current_tick + INPUT_DELAY;

    let local_input = TickInput {
        tick: local_tick,
        player: local_player.id,
        commands: cmds,
    };

    let encoded = encode_tick_input(&local_input);
    client.send_message(DefaultChannel::ReliableOrdered, encoded);

    // Store own commands locally
    command_buffer.pending
        .entry((local_input.tick, local_input.player))
        .or_default()
        .extend(local_input.commands);

    // Receive commands from host
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        if let Some(remote_input) = decode_tick_input(&message) {
            command_buffer.pending
                .entry((remote_input.tick, remote_input.player))
                .or_default()
                .extend(remote_input.commands);
        }
    }
}

/// Gate the simulation: pause virtual time when waiting for remote input
pub fn lockstep_gate_system(
    command_buffer: Res<CommandBuffer>,
    mut time: ResMut<Time<Virtual>>,
) {
    if !command_buffer.has_commands_for_tick(command_buffer.current_tick) {
        time.pause();
    } else {
        time.unpause();
    }
}

// --- Apply Commands System (runs first in FixedUpdate) ---

pub fn apply_commands_system(
    mut command_buffer: ResMut<CommandBuffer>,
    mut commands: Commands,
    mut stable_id_map: ResMut<StableIdMap>,
    mut all_resources: ResMut<AllTeamResources>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    models: Res<ModelLibrary>,
    mut next_stable_id: ResMut<NextStableId>,
    mut factories: Query<&mut Factory>,
    commander_query: Query<&Transform, With<Commander>>,
    terrain: Res<TerrainHeightmap>,
) {
    let tick = command_buffer.current_tick;

    for player in 0..2u8 {
        let player_commands = command_buffer.take_commands(tick, player);

        for cmd in player_commands {
            match cmd {
                GameCommand::MoveUnits { unit_ids, target } => {
                    let target_vec = Vec2::new(target.0, target.1);
                    for uid in &unit_ids {
                        if let Some(entity) = stable_id_map.get(*uid) {
                            commands.entity(entity)
                                .insert(MoveTarget(target_vec))
                                .remove::<AttackTarget>()
                                .remove::<ReclaimTarget>()
                                .remove::<BuildTarget>();
                        }
                    }
                }
                GameCommand::AttackUnits { unit_ids, target_id } => {
                    if let Some(target_entity) = stable_id_map.get(target_id) {
                        for uid in &unit_ids {
                            if let Some(entity) = stable_id_map.get(*uid) {
                                commands.entity(entity)
                                    .insert(AttackTarget(target_entity))
                                    .remove::<MoveTarget>()
                                    .remove::<ReclaimTarget>()
                                    .remove::<BuildTarget>();
                            }
                        }
                    }
                }
                GameCommand::PlaceBuilding { building_type, position, commander_ids } => {
                    let Some(btype) = BuildingType::from_u8(building_type) else { continue };
                    let pos = Vec2::new(position.0, position.1);
                    let bs = btype.stats();
                    let team_id = player as usize;

                    if all_resources.teams[team_id].metal < bs.metal_cost
                        || all_resources.teams[team_id].energy < bs.energy_cost
                    {
                        continue;
                    }
                    all_resources.teams[team_id].metal -= bs.metal_cost;
                    all_resources.teams[team_id].energy -= bs.energy_cost;

                    let building_entity = spawn_building_entity(
                        &mut commands, &mut meshes, &mut materials, pos, btype,
                        player, false, &models, &mut next_stable_id, &mut stable_id_map,
                    );

                    for cid in &commander_ids {
                        if let Some(cmd_entity) = stable_id_map.get(*cid) {
                            commands.entity(cmd_entity)
                                .insert(BuildTarget(building_entity))
                                .remove::<MoveTarget>()
                                .remove::<AttackTarget>()
                                .remove::<ReclaimTarget>();
                        }
                    }
                }
                GameCommand::QueueUnit { factory_id, unit_type } => {
                    let Some(utype) = UnitType::from_u8(unit_type) else { continue };
                    if let Some(factory_entity) = stable_id_map.get(factory_id) {
                        if let Ok(mut factory) = factories.get_mut(factory_entity) {
                            factory.queue.push(utype);
                        }
                    }
                }
                GameCommand::DGun { commander_id, target_pos } => {
                    let team_id = player as usize;
                    if all_resources.teams[team_id].energy < DGUN_ENERGY_COST {
                        continue;
                    }
                    all_resources.teams[team_id].energy -= DGUN_ENERGY_COST;

                    if let Some(cmd_entity) = stable_id_map.get(commander_id) {
                        if let Ok(cmd_tf) = commander_query.get(cmd_entity) {
                            let cmd_pos = game_xy(&cmd_tf.translation);
                            let tgt = Vec2::new(target_pos.0, target_pos.1);
                            let dist = cmd_pos.distance(tgt);
                            if dist <= DGUN_RANGE {
                                let direction = (tgt - cmd_pos).normalize_or_zero();
                                let end_pos = cmd_pos + direction * DGUN_RANGE;

                                let target_entity = commands
                                    .spawn((
                                        Transform::from_translation(game_pos(end_pos.x, end_pos.y, terrain.height_at(end_pos.x, end_pos.y))),
                                        Unit {
                                            hp: 1.0, max_hp: 1.0, speed: 0.0,
                                            attack_damage: 0.0, attack_range: 0.0,
                                            attack_cooldown: 999.0, cooldown_timer: 0.0,
                                            min_attack_range: 0.0, radius: 0.0,
                                        },
                                        Visibility::Hidden,
                                    ))
                                    .id();

                                commands.spawn((
                                    Mesh3d(meshes.add(Sphere::new(4.0))),
                                    MeshMaterial3d(materials.add(StandardMaterial {
                                        base_color: Color::srgb(1.0, 1.0, 0.0),
                                        emissive: LinearRgba::new(5.0, 5.0, 0.0, 1.0),
                                        unlit: true,
                                        ..default()
                                    })),
                                    Transform::from_translation(game_pos(cmd_pos.x, cmd_pos.y, terrain.height_at(cmd_pos.x, cmd_pos.y) + 1.5)),
                                    Projectile {
                                        target: target_entity,
                                        damage: 9999.0,
                                        speed: PROJECTILE_SPEED * 1.5,
                                        is_dgun: true,
                                    },
                                ));
                            }
                        }
                    }
                }
                GameCommand::Reclaim { commander_id, target_id } => {
                    if let Some(cmd_entity) = stable_id_map.get(commander_id) {
                        if let Some(target_entity) = stable_id_map.get(target_id) {
                            commands.entity(cmd_entity)
                                .insert(ReclaimTarget(target_entity))
                                .remove::<MoveTarget>()
                                .remove::<AttackTarget>()
                                .remove::<BuildTarget>();
                        }
                    }
                }
            }
        }
    }

    command_buffer.current_tick += 1;
}

// --- Desync Detection (Phase 6) ---

/// Hash game state for desync detection
pub fn desync_check_system(
    command_buffer: Res<CommandBuffer>,
    units: Query<(&StableId, &Transform, &Unit)>,
    net_role: Res<NetRole>,
    mut server: Option<ResMut<RenetServer>>,
    mut client: Option<ResMut<RenetClient>>,
) {
    if *net_role == NetRole::Singleplayer {
        return;
    }

    let tick = command_buffer.current_tick;
    if tick % 30 != 0 {
        return;
    }

    // Compute simple hash of game state
    let mut sorted: Vec<_> = units.iter().collect();
    sorted.sort_by_key(|(sid, _, _)| sid.0);

    let mut hash: u64 = 0;
    for (sid, tf, unit) in &sorted {
        hash = hash.wrapping_mul(31).wrapping_add(sid.0);
        hash = hash.wrapping_mul(31).wrapping_add((tf.translation.x * 100.0) as u64);
        hash = hash.wrapping_mul(31).wrapping_add((tf.translation.z * 100.0) as u64);
        hash = hash.wrapping_mul(31).wrapping_add((unit.hp * 100.0) as u64);
    }

    let msg = format!("SYNC:{}:{}", tick, hash);
    let bytes: Vec<u8> = msg.into_bytes();

    if let Some(ref mut server) = server {
        server.broadcast_message(DefaultChannel::Unreliable, bytes);
    } else if let Some(ref mut client) = client {
        client.send_message(DefaultChannel::Unreliable, bytes);
    }
}

/// Receive and check desync hashes
pub fn desync_receive_system(
    mut server: Option<ResMut<RenetServer>>,
    mut client: Option<ResMut<RenetClient>>,
    units: Query<(&StableId, &Transform, &Unit)>,
) {
    let mut remote_msgs: Vec<String> = Vec::new();

    if let Some(ref mut server) = server {
        for client_id in server.clients_id() {
            while let Some(message) = server.receive_message(client_id, DefaultChannel::Unreliable) {
                if let Ok(msg) = String::from_utf8(message.to_vec()) {
                    remote_msgs.push(msg);
                }
            }
        }
    } else if let Some(ref mut client) = client {
        while let Some(message) = client.receive_message(DefaultChannel::Unreliable) {
            if let Ok(msg) = String::from_utf8(message.to_vec()) {
                remote_msgs.push(msg);
            }
        }
    }

    for msg in remote_msgs {
        if let Some(rest) = msg.strip_prefix("SYNC:") {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() == 2 {
                if let (Ok(tick), Ok(remote_hash)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                    let mut sorted: Vec<_> = units.iter().collect();
                    sorted.sort_by_key(|(sid, _, _)| sid.0);
                    let mut local_hash: u64 = 0;
                    for (sid, tf, unit) in &sorted {
                        local_hash = local_hash.wrapping_mul(31).wrapping_add(sid.0);
                        local_hash = local_hash.wrapping_mul(31).wrapping_add((tf.translation.x * 100.0) as u64);
                        local_hash = local_hash.wrapping_mul(31).wrapping_add((tf.translation.z * 100.0) as u64);
                        local_hash = local_hash.wrapping_mul(31).wrapping_add((unit.hp * 100.0) as u64);
                    }

                    if local_hash != remote_hash {
                        warn!("DESYNC at tick {}: local={} remote={}", tick, local_hash, remote_hash);
                    }
                }
            }
        }
    }
}
