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

/// Run condition: only run FixedUpdate simulation when commands are available
/// for the current tick (or in singleplayer mode where commands are always pushed).
pub fn simulation_ready(
    command_buffer: Res<CommandBuffer>,
    net_role: Res<NetRole>,
) -> bool {
    match *net_role {
        NetRole::Singleplayer => true,
        _ => command_buffer.has_commands_for_tick(command_buffer.current_tick),
    }
}

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

/// Tracks whether both players are connected and the game can start
#[derive(Resource)]
pub struct MultiplayerReady(pub bool);

impl Default for MultiplayerReady {
    fn default() -> Self {
        Self(false)
    }
}

fn preseed_command_buffer(command_buffer: &mut CommandBuffer) {
    for tick in 0..INPUT_DELAY {
        command_buffer.pending.entry((tick, 0)).or_default();
        command_buffer.pending.entry((tick, 1)).or_default();
    }
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
    mut ready: ResMut<MultiplayerReady>,
    mut command_buffer: ResMut<CommandBuffer>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                info!("Client {} connected", client_id);
                if !ready.0 {
                    ready.0 = true;
                    preseed_command_buffer(&mut command_buffer);
                    info!("Game starting!");
                }
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
    ready: Res<MultiplayerReady>,
) {
    // Always drain incoming messages (even before ready) so renet doesn't stall
    let mut received: Vec<TickInput> = Vec::new();
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
            if let Some(remote_input) = decode_tick_input(&message) {
                received.push(remote_input);
            }
        }
    }

    if !ready.0 {
        // Discard local commands while waiting for connection
        local_commands.commands.clear();
        return;
    }

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

    // Store received remote commands
    for remote_input in received {
        let relay = encode_tick_input(&remote_input);
        command_buffer.pending
            .entry((remote_input.tick, remote_input.player))
            .or_default()
            .extend(remote_input.commands);
        server.broadcast_message(DefaultChannel::ReliableOrdered, relay);
    }
}

/// Client: send local commands and receive commands from host
pub fn client_network_sync(
    mut local_commands: ResMut<LocalCommands>,
    mut command_buffer: ResMut<CommandBuffer>,
    mut client: ResMut<RenetClient>,
    local_player: Res<LocalPlayer>,
    mut ready: ResMut<MultiplayerReady>,
) {
    if !client.is_connected() {
        local_commands.commands.clear();
        return;
    }

    // Detect connection and preseed
    if !ready.0 {
        ready.0 = true;
        preseed_command_buffer(&mut command_buffer);
        info!("Connected to host, game starting!");
    }

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

    // Receive commands from host (skip own player to avoid duplicates from relay)
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        if let Some(remote_input) = decode_tick_input(&message) {
            if remote_input.player != local_player.id {
                command_buffer.pending
                    .entry((remote_input.tick, remote_input.player))
                    .or_default()
                    .extend(remote_input.commands);
            }
        }
    }
}

/// Gate the simulation: pause virtual time when waiting for connection or remote input
pub fn lockstep_gate_system(
    command_buffer: Res<CommandBuffer>,
    ready: Res<MultiplayerReady>,
    mut time: ResMut<Time<Virtual>>,
) {
    if !ready.0 || !command_buffer.has_commands_for_tick(command_buffer.current_tick) {
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
                                .remove::<BuildTarget>()
                                .remove::<Path>();
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
                                    .remove::<BuildTarget>()
                                    .remove::<Path>();
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
                                .remove::<ReclaimTarget>()
                                .remove::<Path>();
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
                                .remove::<BuildTarget>()
                                .remove::<Path>();
                        }
                    }
                }
            }
        }
    }

    command_buffer.current_tick += 1;
}

// --- Desync Detection ---

/// Stores local and remote hashes keyed by tick for proper comparison
#[derive(Resource, Default)]
pub struct SyncHashes {
    pub local: HashMap<u64, u64>,
    pub remote: HashMap<u64, u64>,
}

fn compute_state_hash(units: &Query<(&StableId, &Transform, &Unit)>) -> u64 {
    let mut sorted: Vec<_> = units.iter().collect();
    sorted.sort_by_key(|(sid, _, _)| sid.0);
    let mut hash: u64 = 0;
    for (sid, tf, unit) in &sorted {
        hash = hash.wrapping_mul(31).wrapping_add(sid.0);
        hash = hash.wrapping_mul(31).wrapping_add((tf.translation.x * 100.0) as u64);
        hash = hash.wrapping_mul(31).wrapping_add((tf.translation.z * 100.0) as u64);
        hash = hash.wrapping_mul(31).wrapping_add((unit.hp * 100.0) as u64);
    }
    hash
}

/// Compute local hash and send it; also compare any matching remote hashes
pub fn desync_check_system(
    command_buffer: Res<CommandBuffer>,
    units: Query<(&StableId, &Transform, &Unit)>,
    net_role: Res<NetRole>,
    mut sync_hashes: ResMut<SyncHashes>,
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

    let hash = compute_state_hash(&units);
    sync_hashes.local.insert(tick, hash);

    let msg = format!("SYNC:{}:{}", tick, hash);
    let bytes: Vec<u8> = msg.into_bytes();

    if let Some(ref mut server) = server {
        server.broadcast_message(DefaultChannel::Unreliable, bytes);
    } else if let Some(ref mut client) = client {
        client.send_message(DefaultChannel::Unreliable, bytes);
    }

    // Compare against remote hash for the same tick (if we have it)
    if let Some(&remote_hash) = sync_hashes.remote.get(&tick) {
        if hash != remote_hash {
            warn!("DESYNC at tick {}: local={} remote={}", tick, hash, remote_hash);
        }
        sync_hashes.local.remove(&tick);
        sync_hashes.remote.remove(&tick);
    }
}

/// Receive remote hashes and compare against stored local hashes
pub fn desync_receive_system(
    mut sync_hashes: ResMut<SyncHashes>,
    mut server: Option<ResMut<RenetServer>>,
    mut client: Option<ResMut<RenetClient>>,
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
                    // Compare against local hash if we already computed it for this tick
                    if let Some(&local_hash) = sync_hashes.local.get(&tick) {
                        if local_hash != remote_hash {
                            warn!("DESYNC at tick {}: local={} remote={}", tick, local_hash, remote_hash);
                        }
                        sync_hashes.local.remove(&tick);
                    } else {
                        // Store for later comparison when we reach this tick
                        sync_hashes.remote.insert(tick, remote_hash);
                    }
                }
            }
        }
    }

    // Clean up old entries (more than 300 ticks behind)
    let cutoff = sync_hashes.local.keys().copied().max().unwrap_or(0).saturating_sub(300);
    sync_hashes.local.retain(|&tick, _| tick >= cutoff);
    sync_hashes.remote.retain(|&tick, _| tick >= cutoff);
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- CommandBuffer tests ---

    #[test]
    fn empty_buffer_has_no_commands() {
        let buf = CommandBuffer::default();
        assert!(!buf.has_commands_for_tick(0));
        assert!(!buf.has_commands_for_tick(1));
        assert!(!buf.has_commands_for_tick(999));
    }

    #[test]
    fn push_singleplayer_creates_entries_for_both_players() {
        let mut buf = CommandBuffer::default();
        buf.push_singleplayer(vec![GameCommand::MoveUnits {
            unit_ids: vec![1],
            target: (10.0, 20.0),
        }]);
        assert!(buf.has_commands_for_tick(0));
        // Player 0 has the command
        assert!(buf.pending.get(&(0, 0)).unwrap().len() == 1);
        // Player 1 has an empty entry (AI)
        assert!(buf.pending.get(&(0, 1)).unwrap().is_empty());
    }

    #[test]
    fn push_singleplayer_empty_commands() {
        let mut buf = CommandBuffer::default();
        buf.push_singleplayer(vec![]);
        assert!(buf.has_commands_for_tick(0));
        assert!(buf.pending.get(&(0, 0)).unwrap().is_empty());
        assert!(buf.pending.get(&(0, 1)).unwrap().is_empty());
    }

    #[test]
    fn take_commands_removes_entry() {
        let mut buf = CommandBuffer::default();
        buf.push_singleplayer(vec![GameCommand::MoveUnits {
            unit_ids: vec![1, 2],
            target: (5.0, 5.0),
        }]);

        let cmds = buf.take_commands(0, 0);
        assert_eq!(cmds.len(), 1);

        // Second call returns empty
        let cmds2 = buf.take_commands(0, 0);
        assert!(cmds2.is_empty());

        // Player 1 entry still exists
        assert!(buf.pending.contains_key(&(0, 1)));
    }

    #[test]
    fn take_commands_nonexistent_tick() {
        let mut buf = CommandBuffer::default();
        let cmds = buf.take_commands(42, 0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn has_commands_requires_both_players() {
        let mut buf = CommandBuffer::default();
        buf.pending.entry((5, 0)).or_default();
        assert!(!buf.has_commands_for_tick(5));

        buf.pending.entry((5, 1)).or_default();
        assert!(buf.has_commands_for_tick(5));
    }

    #[test]
    fn preseed_fills_input_delay_ticks() {
        let mut buf = CommandBuffer::default();
        preseed_command_buffer(&mut buf);
        for tick in 0..INPUT_DELAY {
            assert!(
                buf.has_commands_for_tick(tick),
                "tick {} should have commands after preseed",
                tick
            );
        }
        assert!(!buf.has_commands_for_tick(INPUT_DELAY));
    }

    // --- BuildingType to_u8/from_u8 ---

    #[test]
    fn building_type_roundtrip() {
        let variants = [
            BuildingType::MetalExtractor,
            BuildingType::SolarCollector,
            BuildingType::Factory,
            BuildingType::LLT,
            BuildingType::Wall,
            BuildingType::RadarTower,
        ];
        for bt in &variants {
            let v = bt.to_u8();
            let recovered = BuildingType::from_u8(v).expect("should round-trip");
            assert_eq!(*bt, recovered);
        }
    }

    #[test]
    fn building_type_invalid_returns_none() {
        assert!(BuildingType::from_u8(6).is_none());
        assert!(BuildingType::from_u8(255).is_none());
    }

    // --- UnitType to_u8/from_u8 ---

    #[test]
    fn unit_type_roundtrip() {
        let variants = [
            UnitType::Scout,
            UnitType::Raider,
            UnitType::Tank,
            UnitType::Assault,
            UnitType::Artillery,
        ];
        for ut in &variants {
            let v = ut.to_u8();
            let recovered = UnitType::from_u8(v).expect("should round-trip");
            assert_eq!(*ut, recovered);
        }
    }

    #[test]
    fn unit_type_invalid_returns_none() {
        assert!(UnitType::from_u8(5).is_none());
        assert!(UnitType::from_u8(255).is_none());
    }

    // --- Serialization round-trip ---

    #[test]
    fn encode_decode_move_command() {
        let input = TickInput {
            tick: 42,
            player: 1,
            commands: vec![GameCommand::MoveUnits {
                unit_ids: vec![10, 20, 30],
                target: (100.5, 200.5),
            }],
        };
        let bytes = encode_tick_input(&input);
        let decoded = decode_tick_input(&bytes).expect("should decode");
        assert_eq!(decoded.tick, 42);
        assert_eq!(decoded.player, 1);
        assert_eq!(decoded.commands.len(), 1);
        match &decoded.commands[0] {
            GameCommand::MoveUnits { unit_ids, target } => {
                assert_eq!(unit_ids, &[10, 20, 30]);
                assert_eq!(*target, (100.5, 200.5));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn encode_decode_attack_command() {
        let input = TickInput {
            tick: 7,
            player: 0,
            commands: vec![GameCommand::AttackUnits {
                unit_ids: vec![1],
                target_id: 99,
            }],
        };
        let bytes = encode_tick_input(&input);
        let decoded = decode_tick_input(&bytes).unwrap();
        match &decoded.commands[0] {
            GameCommand::AttackUnits { unit_ids, target_id } => {
                assert_eq!(unit_ids, &[1]);
                assert_eq!(*target_id, 99);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn encode_decode_place_building() {
        let input = TickInput {
            tick: 0,
            player: 0,
            commands: vec![GameCommand::PlaceBuilding {
                building_type: BuildingType::Factory.to_u8(),
                position: (500.0, 600.0),
                commander_ids: vec![1, 2],
            }],
        };
        let bytes = encode_tick_input(&input);
        let decoded = decode_tick_input(&bytes).unwrap();
        match &decoded.commands[0] {
            GameCommand::PlaceBuilding { building_type, position, commander_ids } => {
                assert_eq!(BuildingType::from_u8(*building_type), Some(BuildingType::Factory));
                assert_eq!(*position, (500.0, 600.0));
                assert_eq!(commander_ids, &[1, 2]);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn encode_decode_queue_unit() {
        let input = TickInput {
            tick: 10,
            player: 1,
            commands: vec![GameCommand::QueueUnit {
                factory_id: 55,
                unit_type: UnitType::Artillery.to_u8(),
            }],
        };
        let bytes = encode_tick_input(&input);
        let decoded = decode_tick_input(&bytes).unwrap();
        match &decoded.commands[0] {
            GameCommand::QueueUnit { factory_id, unit_type } => {
                assert_eq!(*factory_id, 55);
                assert_eq!(UnitType::from_u8(*unit_type), Some(UnitType::Artillery));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn encode_decode_dgun() {
        let input = TickInput {
            tick: 3,
            player: 0,
            commands: vec![GameCommand::DGun {
                commander_id: 1,
                target_pos: (300.0, 400.0),
            }],
        };
        let bytes = encode_tick_input(&input);
        let decoded = decode_tick_input(&bytes).unwrap();
        match &decoded.commands[0] {
            GameCommand::DGun { commander_id, target_pos } => {
                assert_eq!(*commander_id, 1);
                assert_eq!(*target_pos, (300.0, 400.0));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn encode_decode_reclaim() {
        let input = TickInput {
            tick: 5,
            player: 0,
            commands: vec![GameCommand::Reclaim {
                commander_id: 1,
                target_id: 42,
            }],
        };
        let bytes = encode_tick_input(&input);
        let decoded = decode_tick_input(&bytes).unwrap();
        match &decoded.commands[0] {
            GameCommand::Reclaim { commander_id, target_id } => {
                assert_eq!(*commander_id, 1);
                assert_eq!(*target_id, 42);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn encode_decode_multiple_commands() {
        let input = TickInput {
            tick: 100,
            player: 0,
            commands: vec![
                GameCommand::MoveUnits { unit_ids: vec![1], target: (0.0, 0.0) },
                GameCommand::AttackUnits { unit_ids: vec![2], target_id: 3 },
                GameCommand::DGun { commander_id: 1, target_pos: (50.0, 50.0) },
            ],
        };
        let bytes = encode_tick_input(&input);
        let decoded = decode_tick_input(&bytes).unwrap();
        assert_eq!(decoded.commands.len(), 3);
    }

    #[test]
    fn encode_decode_empty_commands() {
        let input = TickInput {
            tick: 0,
            player: 0,
            commands: vec![],
        };
        let bytes = encode_tick_input(&input);
        let decoded = decode_tick_input(&bytes).unwrap();
        assert_eq!(decoded.tick, 0);
        assert!(decoded.commands.is_empty());
    }

    #[test]
    fn decode_garbage_returns_none() {
        assert!(decode_tick_input(&[]).is_none());
        assert!(decode_tick_input(&[0xFF, 0xFE, 0xFD]).is_none());
        assert!(decode_tick_input(b"not bincode data at all").is_none());
    }
}
