use bevy::prelude::*;
use std::collections::HashMap;

// --- Coordinate Helpers ---

/// Convert 2D game coordinates to 3D world position.
/// Game (x, y) maps to world (x, layer, -y) with Y-up.
pub fn game_pos(x: f32, y: f32, layer: f32) -> Vec3 {
    Vec3::new(x, layer, -y)
}

/// Extract 2D game coordinates from a 3D world position.
pub fn game_xy(pos: &Vec3) -> Vec2 {
    Vec2::new(pos.x, -pos.z)
}

/// Snap a building center to the build grid. Buildings with an even number of
/// grid cells have their center between grid lines (offset by half grid).
pub fn snap_to_build_grid(pos: Vec2, size: (f32, f32)) -> Vec2 {
    let grid = BUILD_GRID_SIZE;
    let cells_x = (size.0 / grid).ceil() as i32;
    let cells_y = (size.1 / grid).ceil() as i32;
    let offset_x = if cells_x % 2 == 0 { 0.0 } else { grid * 0.5 };
    let offset_y = if cells_y % 2 == 0 { 0.0 } else { grid * 0.5 };
    Vec2::new(
        ((pos.x - offset_x) / grid).round() * grid + offset_x,
        ((pos.y - offset_y) / grid).round() * grid + offset_y,
    )
}

// --- Constants ---
pub const MAP_SIZE: f32 = 2000.0;
pub const CAMERA_SPEED: f32 = 500.0;
pub const ZOOM_SPEED: f32 = 0.1;
pub const MIN_ZOOM: f32 = 0.3;
pub const MAX_ZOOM: f32 = 5.0;

pub const BUILD_RANGE: f32 = 150.0;
pub const DGUN_RANGE: f32 = 120.0;
pub const DGUN_ENERGY_COST: f32 = 500.0;
pub const COMMANDER_DEATH_DAMAGE: f32 = 500.0;
pub const COMMANDER_DEATH_RADIUS: f32 = 200.0;
pub const WRECKAGE_DECAY_TIME: f32 = 60.0;
pub const RECLAIM_RANGE: f32 = 60.0;
pub const RECLAIM_TIME: f32 = 2.0;
pub const PROJECTILE_SPEED: f32 = 400.0;
pub const PROJECTILE_SIZE: f32 = 4.0;
pub const MINIMAP_SIZE: f32 = 180.0;
pub const EXTRACTOR_SNAP_RANGE: f32 = 50.0;
pub const BUILD_GRID_SIZE: f32 = 16.0;
pub const EXTRACTOR_INCOME: f32 = 1.5;
pub const SOLAR_INCOME: f32 = 12.0;
pub const COMMANDER_METAL_INCOME: f32 = 1.0;
pub const COMMANDER_ENERGY_INCOME: f32 = 15.0;
pub const RADAR_RANGE: f32 = 600.0;
pub const NAV_GRID_SIZE: usize = 125; // MAP_SIZE 2000 / BUILD_GRID_SIZE 16

// --- Unit Stats Table ---

pub struct UnitStats {
    pub name: &'static str,
    pub hp: f32,
    pub speed: f32,
    pub attack_damage: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub min_attack_range: f32,
    pub metal_cost: f32,
    pub energy_cost: f32,
    pub build_time: f32,
    pub sight_range: f32,
    pub radius: f32,
    pub model_file: &'static str,
    pub model_scale: f32,
}

pub const COMMANDER_STATS: UnitStats = UnitStats {
    name: "Commander",
    hp: 500.0, speed: 80.0, attack_damage: 30.0, attack_range: 150.0,
    attack_cooldown: 1.0, min_attack_range: 0.0,
    metal_cost: 0.0, energy_cost: 0.0, build_time: 0.0,
    sight_range: 300.0, radius: 20.0,
    model_file: "armcom", model_scale: 0.4,
};

const SCOUT_STATS: UnitStats = UnitStats {
    name: "Scout",
    hp: 80.0, speed: 200.0, attack_damage: 5.0, attack_range: 100.0,
    attack_cooldown: 0.5, min_attack_range: 0.0,
    metal_cost: 55.0, energy_cost: 500.0, build_time: 8.0,
    sight_range: 400.0, radius: 10.0,
    model_file: "armpeep", model_scale: 0.35,
};

const RAIDER_STATS: UnitStats = UnitStats {
    name: "Raider",
    hp: 120.0, speed: 160.0, attack_damage: 15.0, attack_range: 130.0,
    attack_cooldown: 0.6, min_attack_range: 0.0,
    metal_cost: 110.0, energy_cost: 600.0, build_time: 10.0,
    sight_range: 250.0, radius: 12.0,
    model_file: "armflash", model_scale: 0.35,
};

const TANK_STATS: UnitStats = UnitStats {
    name: "Tank",
    hp: 200.0, speed: 120.0, attack_damage: 20.0, attack_range: 200.0,
    attack_cooldown: 0.8, min_attack_range: 0.0,
    metal_cost: 225.0, energy_cost: 1500.0, build_time: 15.0,
    sight_range: 250.0, radius: 14.0,
    model_file: "armstump", model_scale: 0.35,
};

const ASSAULT_STATS: UnitStats = UnitStats {
    name: "Assault",
    hp: 400.0, speed: 60.0, attack_damage: 40.0, attack_range: 150.0,
    attack_cooldown: 1.2, min_attack_range: 0.0,
    metal_cost: 400.0, energy_cost: 4000.0, build_time: 20.0,
    sight_range: 250.0, radius: 16.0,
    model_file: "armbull", model_scale: 0.35,
};

const ARTILLERY_STATS: UnitStats = UnitStats {
    name: "Artillery",
    hp: 100.0, speed: 50.0, attack_damage: 60.0, attack_range: 400.0,
    attack_cooldown: 2.0, min_attack_range: 150.0,
    metal_cost: 130.0, energy_cost: 1000.0, build_time: 12.0,
    sight_range: 250.0, radius: 13.0,
    model_file: "armham", model_scale: 0.35,
};

// --- Building Stats Table ---

pub struct BuildingStats {
    pub metal_cost: f32,
    pub energy_cost: f32,
    pub build_time: f32,
    pub size: (f32, f32),
    pub hp: f32,
    pub attack_damage: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub sight_range: f32,
    pub model_file: &'static str,
    pub model_scale: f32,
}

const EXTRACTOR_STATS: BuildingStats = BuildingStats {
    metal_cost: 50.0, energy_cost: 300.0, build_time: 5.0,
    size: (32.0, 32.0), hp: 300.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armmex", model_scale: 0.5,
};

const SOLAR_STATS: BuildingStats = BuildingStats {
    metal_cost: 75.0, energy_cost: 0.0, build_time: 3.0,
    size: (32.0, 32.0), hp: 300.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armsolar", model_scale: 0.5,
};

const FACTORY_STATS: BuildingStats = BuildingStats {
    metal_cost: 500.0, energy_cost: 1000.0, build_time: 15.0,
    size: (48.0, 48.0), hp: 300.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armlab", model_scale: 0.5,
};

const LLT_STATS: BuildingStats = BuildingStats {
    metal_cost: 85.0, energy_cost: 500.0, build_time: 8.0,
    size: (32.0, 32.0), hp: 400.0,
    attack_damage: 25.0, attack_range: 250.0, attack_cooldown: 0.6,
    sight_range: 200.0,
    model_file: "armllt", model_scale: 0.5,
};

const WALL_STATS: BuildingStats = BuildingStats {
    metal_cost: 10.0, energy_cost: 0.0, build_time: 1.0,
    size: (16.0, 16.0), hp: 500.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armdrag", model_scale: 0.5,
};

const RADAR_STATS: BuildingStats = BuildingStats {
    metal_cost: 60.0, energy_cost: 300.0, build_time: 5.0,
    size: (32.0, 32.0), hp: 200.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armrad", model_scale: 0.5,
};

// --- Components ---

#[derive(Component)]
pub struct Unit {
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub attack_damage: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub cooldown_timer: f32,
    pub min_attack_range: f32,
    pub radius: f32,
}

#[derive(Component)]
pub struct Commander;

#[derive(Component)]
pub struct Tank;

#[derive(Component)]
pub struct Scout;

#[derive(Component)]
pub struct Raider;

#[derive(Component)]
pub struct Assault;

#[derive(Component)]
pub struct Artillery;

#[derive(Component, Clone, Copy, Debug)]
pub struct TeamOwned(pub u8);

/// Stable identifier for deterministic network references (Entity handles are local)
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(pub u64);

#[derive(Component)]
pub struct Selected;

#[derive(Component)]
pub struct MoveTarget(pub Vec2);

#[derive(Component)]
pub struct AttackTarget(pub Entity);

#[derive(Component)]
pub struct ReclaimTarget(pub Entity);

#[derive(Component)]
pub struct BuildTarget(pub Entity);

#[derive(Component)]
pub struct NanoParticle {
    pub target: Vec3,
    pub speed: f32,
    pub lifetime: f32,
}

#[derive(Component)]
pub struct Building {
    #[allow(dead_code)]
    pub building_type: BuildingType,
    pub built: bool,
    pub build_progress: f32,
    pub build_time: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BuildingType {
    MetalExtractor,
    SolarCollector,
    Factory,
    LLT,
    Wall,
    RadarTower,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UnitType {
    Scout,
    Raider,
    Tank,
    Assault,
    Artillery,
}

impl UnitType {
    pub fn stats(&self) -> &'static UnitStats {
        match self {
            UnitType::Scout => &SCOUT_STATS,
            UnitType::Raider => &RAIDER_STATS,
            UnitType::Tank => &TANK_STATS,
            UnitType::Assault => &ASSAULT_STATS,
            UnitType::Artillery => &ARTILLERY_STATS,
        }
    }
}

impl BuildingType {
    pub fn stats(&self) -> &'static BuildingStats {
        match self {
            BuildingType::MetalExtractor => &EXTRACTOR_STATS,
            BuildingType::SolarCollector => &SOLAR_STATS,
            BuildingType::Factory => &FACTORY_STATS,
            BuildingType::LLT => &LLT_STATS,
            BuildingType::Wall => &WALL_STATS,
            BuildingType::RadarTower => &RADAR_STATS,
        }
    }
}

#[derive(Component)]
pub struct MetalExtractor;

#[derive(Component)]
pub struct SolarCollector;

#[derive(Component)]
pub struct Factory {
    pub queue: Vec<UnitType>,
    pub produce_timer: f32,
    pub current_build_time: f32,
}

#[derive(Component)]
pub struct LightLaserTower;

#[derive(Component)]
pub struct DragonTeeth;

#[derive(Component)]
pub struct RadarTower;

#[derive(Component)]
pub struct MetalSpot;

#[derive(Component)]
pub struct HealthBarBg;

#[derive(Component)]
pub struct HealthBarFill;

#[derive(Component)]
pub struct BuildGhost;

#[derive(Component)]
pub struct SelectionBox;

#[derive(Component)]
pub struct Projectile {
    pub target: Entity,
    pub damage: f32,
    pub speed: f32,
    pub is_dgun: bool,
}

#[derive(Component)]
pub struct Wreckage {
    pub metal_value: f32,
    pub decay_timer: f32,
}

#[derive(Component)]
pub struct MapFeature {
    pub metal_value: f32,
}

#[derive(Component)]
pub struct SightRange(pub f32);

#[derive(Component)]
pub struct RadarRangeComp(pub f32);

#[derive(Component)]
pub struct HudMetal;

#[derive(Component)]
pub struct HudEnergy;

#[derive(Component)]
pub struct HudBuildHint;

#[derive(Component)]
pub struct HudFactoryQueue;

#[derive(Component)]
pub struct HudNetStatus;

#[derive(Component)]
pub struct GameOverText;

#[derive(Component)]
pub struct DeathExplosion {
    pub timer: f32,
    pub max_radius: f32,
}

#[derive(Component)]
pub struct MinimapFrame;

#[derive(Component)]
pub struct FogOverlay;

#[derive(Component)]
pub struct ExplosionParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
}

#[derive(Component)]
pub struct MuzzleFlash {
    pub lifetime: f32,
}

#[derive(Component)]
pub struct MinimapDot;

// --- Resources ---

#[derive(Resource)]
pub struct GameResources {
    pub metal: f32,
    pub energy: f32,
    pub metal_income: f32,
    pub energy_income: f32,
}

impl Default for GameResources {
    fn default() -> Self {
        Self {
            metal: 1000.0,
            energy: 2000.0,
            metal_income: 0.0,
            energy_income: 0.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct BuildMode {
    pub active: bool,
    pub building_type: Option<BuildingType>,
}

#[derive(Resource, Default)]
pub struct DragSelect {
    pub start: Option<Vec2>,
    pub dragging: bool,
}

#[derive(Resource, Default)]
pub struct CursorWorldPos(pub Vec2);

#[derive(Resource, Default)]
pub struct GameOver(pub Option<String>);

#[derive(Resource, Default)]
pub struct DGunMode(pub bool);

#[derive(Resource)]
pub struct LocalPlayer {
    pub id: u8,
}

impl Default for LocalPlayer {
    fn default() -> Self {
        Self { id: 0 }
    }
}

/// Per-team resources (metal, energy, income)
#[derive(Resource)]
pub struct AllTeamResources {
    pub teams: [GameResources; 2],
}

impl Default for AllTeamResources {
    fn default() -> Self {
        Self {
            teams: [GameResources::default(), GameResources::default()],
        }
    }
}

/// Counter for assigning StableIds
#[derive(Resource)]
pub struct NextStableId(pub u64);

impl Default for NextStableId {
    fn default() -> Self {
        Self(1)
    }
}

/// Map from StableId to Entity for network command resolution
#[derive(Resource, Default)]
pub struct StableIdMap {
    pub map: HashMap<u64, Entity>,
}

impl StableIdMap {
    pub fn insert(&mut self, id: u64, entity: Entity) {
        self.map.insert(id, entity);
    }

    pub fn get(&self, id: u64) -> Option<Entity> {
        self.map.get(&id).copied()
    }

    pub fn remove(&mut self, id: u64) {
        self.map.remove(&id);
    }
}

#[derive(Resource)]
pub struct ModelLibrary {
    pub models: HashMap<String, Handle<Scene>>,
}

impl ModelLibrary {
    pub fn get(&self, model_file: &str, team: u8) -> Option<&Handle<Scene>> {
        let key = format!("{}_{}", model_file, if team == 0 { "blue" } else { "red" });
        self.models.get(&key)
    }
}

// --- Pathfinding ---

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub enum MoveClass {
    Bot,     // Commander, Scout, Artillery — max slope 36°
    Vehicle, // Raider, Tank, Assault — max slope 18°
}

impl MoveClass {
    pub fn max_slope_degrees(&self) -> f32 {
        match self {
            MoveClass::Bot => 36.0,
            MoveClass::Vehicle => 18.0,
        }
    }
}

#[derive(Component)]
pub struct Path {
    pub waypoints: Vec<Vec2>, // game coords, ordered next→final
    pub grid_version: u32,    // for invalidation when NavGrid changes
    pub goal: Vec2,           // target position when path was computed
}

#[derive(Resource)]
pub struct NavGrid {
    pub blocked: Vec<bool>,  // NAV_GRID_SIZE * NAV_GRID_SIZE
    pub slope: Vec<f32>,     // slope angle in degrees per cell
    pub version: u32,        // bumped when blocked state changes
    prev_blocked: Vec<bool>, // for change detection
}

impl NavGrid {
    pub fn new(terrain: &TerrainHeightmap) -> Self {
        let total = NAV_GRID_SIZE * NAV_GRID_SIZE;
        let mut slope = vec![0.0f32; total];

        let cell_size = BUILD_GRID_SIZE;
        for cy in 0..NAV_GRID_SIZE {
            for cx in 0..NAV_GRID_SIZE {
                let wx = cx as f32 * cell_size;
                let wy = cy as f32 * cell_size;
                // Sample 4 corners of the cell
                let h00 = terrain.height_at(wx, wy);
                let h10 = terrain.height_at(wx + cell_size, wy);
                let h01 = terrain.height_at(wx, wy + cell_size);
                let h11 = terrain.height_at(wx + cell_size, wy + cell_size);
                let max_h = h00.max(h10).max(h01).max(h11);
                let min_h = h00.min(h10).min(h01).min(h11);
                let max_diff = max_h - min_h;
                let slope_angle = (max_diff / cell_size).atan().to_degrees();
                slope[cy * NAV_GRID_SIZE + cx] = slope_angle;
            }
        }

        NavGrid {
            blocked: vec![false; total],
            slope,
            version: 0,
            prev_blocked: vec![false; total],
        }
    }

    /// Convert game coordinates to grid cell indices
    pub fn game_to_cell(&self, x: f32, y: f32) -> (usize, usize) {
        let cx = ((x / BUILD_GRID_SIZE) as usize).min(NAV_GRID_SIZE - 1);
        let cy = ((y / BUILD_GRID_SIZE) as usize).min(NAV_GRID_SIZE - 1);
        (cx, cy)
    }

    /// Convert grid cell center to game coordinates
    pub fn cell_to_game(&self, cx: usize, cy: usize) -> Vec2 {
        Vec2::new(
            cx as f32 * BUILD_GRID_SIZE + BUILD_GRID_SIZE * 0.5,
            cy as f32 * BUILD_GRID_SIZE + BUILD_GRID_SIZE * 0.5,
        )
    }

    /// Check if a cell is passable for the given movement class
    pub fn is_passable(&self, cx: usize, cy: usize, move_class: MoveClass) -> bool {
        if cx >= NAV_GRID_SIZE || cy >= NAV_GRID_SIZE {
            return false;
        }
        let idx = cy * NAV_GRID_SIZE + cx;
        if self.blocked[idx] {
            return false;
        }
        self.slope[idx] <= move_class.max_slope_degrees()
    }

    /// Mark/unmark cells covered by a building footprint
    pub fn mark_building(&mut self, center: Vec2, size: (f32, f32), blocked: bool) {
        let half_x = size.0 * 0.5;
        let half_y = size.1 * 0.5;
        let min_x = ((center.x - half_x) / BUILD_GRID_SIZE).floor() as i32;
        let max_x = ((center.x + half_x) / BUILD_GRID_SIZE).ceil() as i32;
        let min_y = ((center.y - half_y) / BUILD_GRID_SIZE).floor() as i32;
        let max_y = ((center.y + half_y) / BUILD_GRID_SIZE).ceil() as i32;
        for cy in min_y..max_y {
            for cx in min_x..max_x {
                if cx >= 0 && cx < NAV_GRID_SIZE as i32 && cy >= 0 && cy < NAV_GRID_SIZE as i32 {
                    self.blocked[(cy as usize) * NAV_GRID_SIZE + cx as usize] = blocked;
                }
            }
        }
    }

    /// Swap blocked state and bump version if changed
    pub fn finish_sync(&mut self) {
        if self.blocked != self.prev_blocked {
            self.version = self.version.wrapping_add(1);
            self.prev_blocked.clone_from(&self.blocked);
        }
    }
}

/// Tracks the walk animation phase for a commander entity
#[derive(Component)]
pub struct CommanderWalkAnim {
    pub phase: f32, // 0..1 walk cycle phase
    pub active: bool,
}

/// Tracks walk animation phase for biped units (armham / artillery)
#[derive(Component)]
pub struct BipedWalkAnim {
    pub phase: f32,
    pub active: bool,
}

/// Marker for vehicle units that have turret aim animation
#[derive(Component)]
pub struct VehicleAnim;

// --- Terrain ---

pub const TERRAIN_GRID_SIZE: usize = 101; // 101x101 vertices for 2000x2000 map (~20 units per cell)
pub const TERRAIN_MAX_HEIGHT: f32 = 30.0; // maximum hill height (for color mapping)
pub const FOG_GRID_SIZE: usize = 51; // coarser grid for fog overlay (performance)

/// Grid-aligned metal spot positions. Each coordinate is snapped to the 16-unit
/// build grid so that 32×32 extractors (2 grid cells) center perfectly.
/// All spots are placed well inside flat terrain zones (plateau interiors,
/// valley floor, or central mesa) — never on slopes or ridge edges.
pub const METAL_SPOT_POSITIONS: [(f32, f32); 10] = [
    // Player plateau (center 300,300 r=350; stay within ~150 of center)
    (208.0, 208.0),
    (400.0, 208.0),
    (208.0, 400.0),
    // Valley floor (between ridges, far from slopes)
    (704.0, 608.0),
    (1000.0, 1000.0), // central mesa center
    (1296.0, 1392.0),
    // Enemy plateau (center 1700,1700 r=350; stay within ~150 of center)
    (1600.0, 1800.0),
    (1800.0, 1600.0),
    (1696.0, 1696.0),
    (1800.0, 1800.0),
];

/// Simple hash-based noise for terrain generation (no external crate needed)
fn hash_noise(x: f32, y: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();

    // Smoothstep for interpolation
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    // Hash function using sine
    let hash = |px: i32, py: i32| -> f32 {
        let n = px.wrapping_mul(127).wrapping_add(py.wrapping_mul(311)) as f32;
        (n * 43758.5453).sin().fract().abs()
    };

    let a = hash(ix, iy);
    let b = hash(ix + 1, iy);
    let c = hash(ix, iy + 1);
    let d = hash(ix + 1, iy + 1);

    // Bilinear interpolation
    let ab = a + (b - a) * ux;
    let cd = c + (d - c) * ux;
    ab + (cd - ab) * uy
}

/// Multi-octave noise for natural-looking terrain
fn terrain_noise(x: f32, y: f32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_amp = 0.0;

    for _ in 0..2 {
        value += hash_noise(x * frequency, y * frequency) * amplitude;
        max_amp += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_amp
}

#[derive(Resource)]
pub struct TerrainHeightmap {
    pub heights: Vec<f32>, // TERRAIN_GRID_SIZE * TERRAIN_GRID_SIZE
    pub cell_size: f32,
}

impl TerrainHeightmap {
    pub fn generate() -> Self {
        let cell_size = MAP_SIZE / (TERRAIN_GRID_SIZE - 1) as f32;
        let mut heights = vec![0.0; TERRAIN_GRID_SIZE * TERRAIN_GRID_SIZE];

        // Smoothstep helper: 0 at edge, 1 at center
        let smoothstep = |t: f32| -> f32 {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };

        // Distance from point to line segment (px,py) -> (ax,ay)-(bx,by)
        let dist_to_segment = |px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32| -> f32 {
            let dx = bx - ax;
            let dy = by - ay;
            let len_sq = dx * dx + dy * dy;
            if len_sq < 0.001 {
                return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
            }
            let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
            let t = t.clamp(0.0, 1.0);
            let cx = ax + t * dx;
            let cy = ay + t * dy;
            ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
        };

        for gy in 0..TERRAIN_GRID_SIZE {
            for gx in 0..TERRAIN_GRID_SIZE {
                let wx = gx as f32 * cell_size;
                let wy = gy as f32 * cell_size;

                // --- Valley floor (base height) ---
                let mut h: f32 = 2.0;

                // --- Player base plateau: center (300,300), radius 350, height 12 ---
                let player_dist = ((wx - 300.0).powi(2) + (wy - 300.0).powi(2)).sqrt();
                let player_t = 1.0 - smoothstep(player_dist / 350.0);
                h = h.max(h + player_t * 10.0); // raises to ~12

                // --- Enemy base plateau: center (1700,1700), radius 350, height 12 ---
                let enemy_dist = ((wx - 1700.0).powi(2) + (wy - 1700.0).powi(2)).sqrt();
                let enemy_t = 1.0 - smoothstep(enemy_dist / 350.0);
                h = h.max(h + enemy_t * 10.0);

                // --- NW ridge: line from (0,900) to (900,0), height ~24, width ~250 ---
                let nw_dist = dist_to_segment(wx, wy, 0.0, 900.0, 900.0, 0.0);
                let nw_t = 1.0 - smoothstep(nw_dist / 250.0);
                h = h.max(2.0 + nw_t * 22.0);

                // --- SE ridge: line from (1100,2000) to (2000,1100), height ~24, width ~250 ---
                let se_dist = dist_to_segment(wx, wy, 1100.0, 2000.0, 2000.0, 1100.0);
                let se_t = 1.0 - smoothstep(se_dist / 250.0);
                h = h.max(2.0 + se_t * 22.0);

                // --- Central mesa: center (1000,1000), radius 150, height ~8 ---
                let center_dist = ((wx - 1000.0).powi(2) + (wy - 1000.0).powi(2)).sqrt();
                let center_t = 1.0 - smoothstep(center_dist / 150.0);
                h = h.max(2.0 + center_t * 6.0);

                // --- Noise overlay: 2 octaves, amplitude 4 (surface texture) ---
                let noise_scale = 0.008;
                let noise = terrain_noise(wx * noise_scale + 3.7, wy * noise_scale + 7.1);
                h += noise * 4.0;

                // --- Edge falloff: flatten within 100 units of map edges ---
                let edge_margin = 100.0;
                let ex = (wx.min(MAP_SIZE - wx) / edge_margin).min(1.0);
                let ey = (wy.min(MAP_SIZE - wy) / edge_margin).min(1.0);
                let edge_factor = smoothstep(ex) * smoothstep(ey);
                h = h * edge_factor;

                // --- Spawn area flatten: 250 radius around (200,200) and (1800,1800) ---
                let spawn_player = ((wx - 200.0).powi(2) + (wy - 200.0).powi(2)).sqrt();
                let spawn_enemy = ((wx - 1800.0).powi(2) + (wy - 1800.0).powi(2)).sqrt();
                let spawn_radius = 250.0;
                // Inside spawn radius: lerp toward plateau height (12)
                let plateau_h = 12.0;
                if spawn_player < spawn_radius {
                    let t = smoothstep(spawn_player / spawn_radius);
                    h = plateau_h * (1.0 - t) + h * t;
                }
                if spawn_enemy < spawn_radius {
                    let t = smoothstep(spawn_enemy / spawn_radius);
                    h = plateau_h * (1.0 - t) + h * t;
                }

                heights[gy * TERRAIN_GRID_SIZE + gx] = h;
            }
        }

        TerrainHeightmap { heights, cell_size }
    }

    /// Get terrain height at game coordinates (x, y) with bilinear interpolation
    pub fn height_at(&self, x: f32, y: f32) -> f32 {
        let gx = x / self.cell_size;
        let gy = y / self.cell_size;

        let ix = (gx.floor() as usize).min(TERRAIN_GRID_SIZE - 2);
        let iy = (gy.floor() as usize).min(TERRAIN_GRID_SIZE - 2);
        let fx = (gx - ix as f32).clamp(0.0, 1.0);
        let fy = (gy - iy as f32).clamp(0.0, 1.0);

        let i00 = iy * TERRAIN_GRID_SIZE + ix;
        let i10 = i00 + 1;
        let i01 = i00 + TERRAIN_GRID_SIZE;
        let i11 = i01 + 1;

        let h00 = self.heights[i00];
        let h10 = self.heights[i10];
        let h01 = self.heights[i01];
        let h11 = self.heights[i11];

        let h0 = h00 + (h10 - h00) * fx;
        let h1 = h01 + (h11 - h01) * fx;
        h0 + (h1 - h0) * fy
    }

    /// Check if terrain is flat enough for building placement.
    /// Checks every build grid cell the building covers; for each cell, samples
    /// the 4 corners and rejects if max height diff exceeds threshold.
    pub fn is_flat_enough(&self, x: f32, y: f32, size_x: f32, size_y: f32) -> bool {
        let grid = BUILD_GRID_SIZE;
        let half_x = size_x * 0.5;
        let half_y = size_y * 0.5;
        let start_x = x - half_x;
        let start_y = y - half_y;
        let cells_x = (size_x / grid).ceil() as i32;
        let cells_y = (size_y / grid).ceil() as i32;

        for cy in 0..cells_y {
            for cx in 0..cells_x {
                let cx0 = start_x + cx as f32 * grid;
                let cy0 = start_y + cy as f32 * grid;
                let h00 = self.height_at(cx0, cy0);
                let h10 = self.height_at(cx0 + grid, cy0);
                let h01 = self.height_at(cx0, cy0 + grid);
                let h11 = self.height_at(cx0 + grid, cy0 + grid);
                let min_h = h00.min(h10).min(h01).min(h11);
                let max_h = h00.max(h10).max(h01).max(h11);
                if (max_h - min_h) > 2.0 {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Coordinate helpers ---

    #[test]
    fn game_pos_known_values() {
        let v = game_pos(100.0, 200.0, 5.0);
        assert_eq!(v.x, 100.0);
        assert_eq!(v.y, 5.0);
        assert_eq!(v.z, -200.0);
    }

    #[test]
    fn game_xy_known_values() {
        let v = game_xy(&Vec3::new(100.0, 5.0, -200.0));
        assert_eq!(v.x, 100.0);
        assert_eq!(v.y, 200.0);
    }

    #[test]
    fn game_pos_game_xy_roundtrip() {
        let cases = [
            (0.0, 0.0),
            (100.0, 200.0),
            (-50.0, 300.0),
            (1999.0, 1.0),
        ];
        for (x, y) in cases {
            let world = game_pos(x, y, 0.0);
            let back = game_xy(&world);
            assert!(
                (back.x - x).abs() < f32::EPSILON && (back.y - y).abs() < f32::EPSILON,
                "roundtrip failed for ({}, {}): got ({}, {})",
                x, y, back.x, back.y,
            );
        }
    }

    #[test]
    fn game_pos_game_xy_roundtrip_with_layer() {
        // Layer (height) is discarded by game_xy, but x/y should survive
        let world = game_pos(42.0, 99.0, 15.0);
        let back = game_xy(&world);
        assert_eq!(back.x, 42.0);
        assert_eq!(back.y, 99.0);
    }

    // --- snap_to_build_grid ---

    #[test]
    fn snap_to_grid_odd_cells() {
        // Wall: 16x16 → 1 cell each axis (odd) → offset by half grid (8)
        let snapped = snap_to_build_grid(Vec2::new(100.0, 100.0), (16.0, 16.0));
        // Should snap to multiples of 16 + 8
        assert_eq!(snapped.x % BUILD_GRID_SIZE, BUILD_GRID_SIZE * 0.5);
        assert_eq!(snapped.y % BUILD_GRID_SIZE, BUILD_GRID_SIZE * 0.5);
    }

    #[test]
    fn snap_to_grid_even_cells() {
        // Extractor: 32x32 → 2 cells each axis (even) → no offset
        let snapped = snap_to_build_grid(Vec2::new(100.0, 100.0), (32.0, 32.0));
        assert!((snapped.x % BUILD_GRID_SIZE).abs() < f32::EPSILON);
        assert!((snapped.y % BUILD_GRID_SIZE).abs() < f32::EPSILON);
    }

    #[test]
    fn snap_to_grid_factory() {
        // Factory: 48x48 → 3 cells (odd) → offset by half grid
        let snapped = snap_to_build_grid(Vec2::new(500.0, 500.0), (48.0, 48.0));
        let remainder_x = snapped.x % BUILD_GRID_SIZE;
        assert!(
            (remainder_x - BUILD_GRID_SIZE * 0.5).abs() < f32::EPSILON,
            "factory x not on half-grid: remainder={}",
            remainder_x,
        );
    }

    #[test]
    fn snap_idempotent() {
        let size = (32.0, 32.0);
        let first = snap_to_build_grid(Vec2::new(137.0, 263.0), size);
        let second = snap_to_build_grid(first, size);
        assert!(
            (first.x - second.x).abs() < f32::EPSILON
                && (first.y - second.y).abs() < f32::EPSILON,
            "snap should be idempotent",
        );
    }

    // --- TerrainHeightmap ---

    #[test]
    fn terrain_generate_correct_dimensions() {
        let terrain = TerrainHeightmap::generate();
        assert_eq!(terrain.heights.len(), TERRAIN_GRID_SIZE * TERRAIN_GRID_SIZE);
    }

    #[test]
    fn terrain_deterministic() {
        let a = TerrainHeightmap::generate();
        let b = TerrainHeightmap::generate();
        assert_eq!(a.heights, b.heights, "terrain generation must be deterministic");
    }

    #[test]
    fn terrain_height_at_in_range() {
        let terrain = TerrainHeightmap::generate();
        // Sample various positions and ensure heights are non-negative and reasonable
        let positions = [
            (0.0, 0.0),
            (200.0, 200.0),
            (1000.0, 1000.0),
            (1800.0, 1800.0),
            (MAP_SIZE, MAP_SIZE),
        ];
        for (x, y) in positions {
            let h = terrain.height_at(x, y);
            assert!(
                h >= 0.0 && h < 50.0,
                "height_at({}, {}) = {} out of expected range",
                x, y, h,
            );
        }
    }

    #[test]
    fn terrain_height_at_clamps_negative() {
        let terrain = TerrainHeightmap::generate();
        // Negative coords should clamp, not panic
        let h = terrain.height_at(-100.0, -100.0);
        assert!(h >= 0.0);
    }

    #[test]
    fn terrain_height_at_clamps_beyond_map() {
        let terrain = TerrainHeightmap::generate();
        // Beyond map should clamp, not panic
        let h = terrain.height_at(MAP_SIZE + 500.0, MAP_SIZE + 500.0);
        assert!(h >= 0.0);
    }

    #[test]
    fn terrain_flat_at_player_spawn() {
        let terrain = TerrainHeightmap::generate();
        // Player spawn area (200,200) should be flat enough for any building
        assert!(
            terrain.is_flat_enough(200.0, 200.0, 48.0, 48.0),
            "player spawn should be flat enough for factory",
        );
    }

    #[test]
    fn terrain_flat_at_enemy_spawn() {
        let terrain = TerrainHeightmap::generate();
        assert!(
            terrain.is_flat_enough(1800.0, 1800.0, 48.0, 48.0),
            "enemy spawn should be flat enough for factory",
        );
    }

    // --- Stats lookups ---

    #[test]
    fn unit_type_stats_values() {
        assert_eq!(UnitType::Scout.stats().hp, 80.0);
        assert_eq!(UnitType::Scout.stats().speed, 200.0);
        assert_eq!(UnitType::Tank.stats().hp, 200.0);
        assert_eq!(UnitType::Artillery.stats().min_attack_range, 150.0);
        assert_eq!(UnitType::Artillery.stats().attack_range, 400.0);
        assert_eq!(UnitType::Assault.stats().hp, 400.0);
        assert_eq!(UnitType::Raider.stats().speed, 160.0);
    }

    #[test]
    fn building_type_stats_values() {
        assert_eq!(BuildingType::MetalExtractor.stats().metal_cost, 50.0);
        assert_eq!(BuildingType::SolarCollector.stats().energy_cost, 0.0);
        assert_eq!(BuildingType::Factory.stats().size, (48.0, 48.0));
        assert_eq!(BuildingType::LLT.stats().attack_damage, 25.0);
        assert_eq!(BuildingType::LLT.stats().attack_range, 250.0);
        assert_eq!(BuildingType::Wall.stats().hp, 500.0);
        assert_eq!(BuildingType::Wall.stats().size, (16.0, 16.0));
        assert_eq!(BuildingType::RadarTower.stats().metal_cost, 60.0);
    }

    #[test]
    fn commander_stats_values() {
        assert_eq!(COMMANDER_STATS.hp, 500.0);
        assert_eq!(COMMANDER_STATS.speed, 80.0);
        assert_eq!(COMMANDER_STATS.attack_damage, 30.0);
    }

    // --- NavGrid ---

    #[test]
    fn navgrid_dimensions() {
        assert_eq!(NAV_GRID_SIZE, 125);
        // MAP_SIZE / BUILD_GRID_SIZE = 2000/16 = 125
    }

    #[test]
    fn navgrid_game_to_cell_and_back() {
        let terrain = TerrainHeightmap::generate();
        let grid = NavGrid::new(&terrain);
        // Cell (0,0) center should be at (8,8)
        let center = grid.cell_to_game(0, 0);
        assert!((center.x - 8.0).abs() < f32::EPSILON);
        assert!((center.y - 8.0).abs() < f32::EPSILON);
        // Converting back
        let (cx, cy) = grid.game_to_cell(center.x, center.y);
        assert_eq!(cx, 0);
        assert_eq!(cy, 0);
    }

    #[test]
    fn navgrid_mark_building() {
        let terrain = TerrainHeightmap::generate();
        let mut grid = NavGrid::new(&terrain);
        // Mark a 32x32 building at center (200, 200)
        let center = Vec2::new(200.0, 200.0);
        grid.mark_building(center, (32.0, 32.0), true);
        let (cx, cy) = grid.game_to_cell(200.0, 200.0);
        assert!(grid.blocked[cy * NAV_GRID_SIZE + cx]);
        // Unmark
        grid.mark_building(center, (32.0, 32.0), false);
        assert!(!grid.blocked[cy * NAV_GRID_SIZE + cx]);
    }

    #[test]
    fn navgrid_version_bumps_on_change() {
        let terrain = TerrainHeightmap::generate();
        let mut grid = NavGrid::new(&terrain);
        let v0 = grid.version;
        grid.mark_building(Vec2::new(100.0, 100.0), (16.0, 16.0), true);
        grid.finish_sync();
        assert_eq!(grid.version, v0.wrapping_add(1));
        // No change → no bump
        grid.finish_sync();
        assert_eq!(grid.version, v0.wrapping_add(1));
    }

    #[test]
    fn navgrid_spawn_area_passable() {
        let terrain = TerrainHeightmap::generate();
        let grid = NavGrid::new(&terrain);
        // Player spawn area (200,200) should be passable for both classes
        let (cx, cy) = grid.game_to_cell(200.0, 200.0);
        assert!(grid.is_passable(cx, cy, MoveClass::Bot));
        assert!(grid.is_passable(cx, cy, MoveClass::Vehicle));
    }

    #[test]
    fn navgrid_out_of_bounds_not_passable() {
        let terrain = TerrainHeightmap::generate();
        let grid = NavGrid::new(&terrain);
        assert!(!grid.is_passable(NAV_GRID_SIZE, 0, MoveClass::Bot));
        assert!(!grid.is_passable(0, NAV_GRID_SIZE, MoveClass::Bot));
    }

    #[test]
    fn move_class_slope_limits() {
        assert_eq!(MoveClass::Bot.max_slope_degrees(), 36.0);
        assert_eq!(MoveClass::Vehicle.max_slope_degrees(), 18.0);
    }

    // --- StableIdMap ---

    #[test]
    fn stable_id_map_insert_and_get() {
        let mut map = StableIdMap::default();
        let entity = Entity::from_bits(42);
        map.insert(1, entity);
        assert_eq!(map.get(1), Some(entity));
    }

    #[test]
    fn stable_id_map_get_missing() {
        let map = StableIdMap::default();
        assert_eq!(map.get(999), None);
    }

    #[test]
    fn stable_id_map_remove() {
        let mut map = StableIdMap::default();
        let entity = Entity::from_bits(7);
        map.insert(5, entity);
        assert!(map.get(5).is_some());
        map.remove(5);
        assert_eq!(map.get(5), None);
    }

    #[test]
    fn stable_id_map_overwrite() {
        let mut map = StableIdMap::default();
        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);
        map.insert(10, e1);
        map.insert(10, e2);
        assert_eq!(map.get(10), Some(e2));
    }

    // --- Resource defaults ---

    #[test]
    fn game_resources_default() {
        let res = GameResources::default();
        assert_eq!(res.metal, 1000.0);
        assert_eq!(res.energy, 2000.0);
        assert_eq!(res.metal_income, 0.0);
        assert_eq!(res.energy_income, 0.0);
    }

    #[test]
    fn all_team_resources_default_two_teams() {
        let all = AllTeamResources::default();
        assert_eq!(all.teams.len(), 2);
        for team_res in &all.teams {
            assert_eq!(team_res.metal, 1000.0);
            assert_eq!(team_res.energy, 2000.0);
        }
    }

    #[test]
    fn next_stable_id_starts_at_one() {
        let nsi = NextStableId::default();
        assert_eq!(nsi.0, 1);
    }

    #[test]
    fn local_player_default_is_zero() {
        let lp = LocalPlayer::default();
        assert_eq!(lp.id, 0);
    }

    // --- Constants sanity ---

    #[test]
    fn constants_positive() {
        assert!(MAP_SIZE > 0.0);
        assert!(CAMERA_SPEED > 0.0);
        assert!(ZOOM_SPEED > 0.0);
        assert!(MIN_ZOOM > 0.0);
        assert!(MAX_ZOOM > MIN_ZOOM);
        assert!(BUILD_RANGE > 0.0);
        assert!(DGUN_RANGE > 0.0);
        assert!(DGUN_ENERGY_COST > 0.0);
        assert!(PROJECTILE_SPEED > 0.0);
        assert!(MINIMAP_SIZE > 0.0);
        assert!(BUILD_GRID_SIZE > 0.0);
        assert!(EXTRACTOR_SNAP_RANGE > 0.0);
        assert!(RECLAIM_RANGE > 0.0);
        assert!(RECLAIM_TIME > 0.0);
        assert!(WRECKAGE_DECAY_TIME > 0.0);
        assert!(RADAR_RANGE > 0.0);
    }

    #[test]
    fn nav_grid_size_matches_map() {
        // NAV_GRID_SIZE should be MAP_SIZE / BUILD_GRID_SIZE
        let expected = (MAP_SIZE / BUILD_GRID_SIZE) as usize;
        assert_eq!(NAV_GRID_SIZE, expected);
    }

    // --- Metal spots ---

    #[test]
    fn metal_spots_within_map_bounds() {
        for &(mx, my) in &METAL_SPOT_POSITIONS {
            assert!(
                mx >= 0.0 && mx <= MAP_SIZE,
                "metal spot x={} out of map bounds",
                mx
            );
            assert!(
                my >= 0.0 && my <= MAP_SIZE,
                "metal spot y={} out of map bounds",
                my
            );
        }
    }

    #[test]
    fn metal_spots_on_half_grid() {
        // Metal spots should be on the half-grid (multiples of BUILD_GRID_SIZE / 2)
        // so extractors can snap precisely to them
        let half = BUILD_GRID_SIZE / 2.0;
        for &(mx, my) in &METAL_SPOT_POSITIONS {
            assert!(
                (mx % half).abs() < f32::EPSILON,
                "metal spot x={} not on half-grid (% {} = {})",
                mx, half, mx % half,
            );
            assert!(
                (my % half).abs() < f32::EPSILON,
                "metal spot y={} not on half-grid (% {} = {})",
                my, half, my % half,
            );
        }
    }

    #[test]
    fn metal_spots_terrain_flat_for_extractor() {
        let terrain = TerrainHeightmap::generate();
        let ext_size = BuildingType::MetalExtractor.stats().size;
        for &(mx, my) in &METAL_SPOT_POSITIONS {
            assert!(
                terrain.is_flat_enough(mx, my, ext_size.0, ext_size.1),
                "metal spot ({}, {}) terrain not flat enough for extractor",
                mx, my,
            );
        }
    }

    // --- Unit stats consistency ---

    #[test]
    fn all_unit_types_have_positive_hp() {
        for ut in &[UnitType::Scout, UnitType::Raider, UnitType::Tank, UnitType::Assault, UnitType::Artillery] {
            let s = ut.stats();
            assert!(s.hp > 0.0, "{} has non-positive HP", s.name);
        }
        assert!(COMMANDER_STATS.hp > 0.0);
    }

    #[test]
    fn all_unit_types_have_positive_speed() {
        for ut in &[UnitType::Scout, UnitType::Raider, UnitType::Tank, UnitType::Assault, UnitType::Artillery] {
            let s = ut.stats();
            assert!(s.speed > 0.0, "{} has non-positive speed", s.name);
        }
        assert!(COMMANDER_STATS.speed > 0.0);
    }

    #[test]
    fn all_unit_types_have_positive_radius() {
        for ut in &[UnitType::Scout, UnitType::Raider, UnitType::Tank, UnitType::Assault, UnitType::Artillery] {
            let s = ut.stats();
            assert!(s.radius > 0.0, "{} has non-positive radius", s.name);
        }
        assert!(COMMANDER_STATS.radius > 0.0);
    }

    #[test]
    fn all_unit_types_have_attack_range_gte_min() {
        for ut in &[UnitType::Scout, UnitType::Raider, UnitType::Tank, UnitType::Assault, UnitType::Artillery] {
            let s = ut.stats();
            assert!(
                s.attack_range >= s.min_attack_range,
                "{} has attack_range {} < min_attack_range {}",
                s.name, s.attack_range, s.min_attack_range,
            );
        }
    }

    #[test]
    fn factory_produced_units_have_build_cost() {
        // All non-commander units should have metal and energy cost
        for ut in &[UnitType::Scout, UnitType::Raider, UnitType::Tank, UnitType::Assault, UnitType::Artillery] {
            let s = ut.stats();
            assert!(s.metal_cost > 0.0, "{} has no metal cost", s.name);
            assert!(s.energy_cost > 0.0, "{} has no energy cost", s.name);
            assert!(s.build_time > 0.0, "{} has no build time", s.name);
        }
    }

    #[test]
    fn commander_has_no_build_cost() {
        assert_eq!(COMMANDER_STATS.metal_cost, 0.0);
        assert_eq!(COMMANDER_STATS.energy_cost, 0.0);
    }

    #[test]
    fn all_unit_types_have_sight_range() {
        for ut in &[UnitType::Scout, UnitType::Raider, UnitType::Tank, UnitType::Assault, UnitType::Artillery] {
            let s = ut.stats();
            assert!(s.sight_range > 0.0, "{} has no sight range", s.name);
        }
        assert!(COMMANDER_STATS.sight_range > 0.0);
    }

    #[test]
    fn scout_is_fastest_unit() {
        let scout_speed = UnitType::Scout.stats().speed;
        for ut in &[UnitType::Raider, UnitType::Tank, UnitType::Assault, UnitType::Artillery] {
            assert!(
                scout_speed >= ut.stats().speed,
                "scout should be fastest but {} is faster",
                ut.stats().name,
            );
        }
    }

    #[test]
    fn artillery_has_min_attack_range() {
        let arty = UnitType::Artillery.stats();
        assert!(
            arty.min_attack_range > 0.0,
            "artillery should have a minimum attack range",
        );
    }

    // --- Building stats consistency ---

    #[test]
    fn all_building_types_have_positive_hp() {
        let types = [
            BuildingType::MetalExtractor,
            BuildingType::SolarCollector,
            BuildingType::Factory,
            BuildingType::LLT,
            BuildingType::Wall,
            BuildingType::RadarTower,
        ];
        for bt in &types {
            let s = bt.stats();
            assert!(s.hp > 0.0, "{:?} has non-positive HP", bt);
        }
    }

    #[test]
    fn building_sizes_are_grid_aligned() {
        let types = [
            BuildingType::MetalExtractor,
            BuildingType::SolarCollector,
            BuildingType::Factory,
            BuildingType::LLT,
            BuildingType::Wall,
            BuildingType::RadarTower,
        ];
        for bt in &types {
            let s = bt.stats();
            assert!(
                (s.size.0 % BUILD_GRID_SIZE).abs() < f32::EPSILON,
                "{:?} width {} not grid-aligned",
                bt, s.size.0,
            );
            assert!(
                (s.size.1 % BUILD_GRID_SIZE).abs() < f32::EPSILON,
                "{:?} height {} not grid-aligned",
                bt, s.size.1,
            );
        }
    }

    #[test]
    fn only_llt_has_attack() {
        // Among buildings, only LLT should deal damage
        let passive_types = [
            BuildingType::MetalExtractor,
            BuildingType::SolarCollector,
            BuildingType::Factory,
            BuildingType::Wall,
            BuildingType::RadarTower,
        ];
        for bt in &passive_types {
            assert_eq!(
                bt.stats().attack_damage, 0.0,
                "{:?} should not have attack damage",
                bt,
            );
        }
        assert!(BuildingType::LLT.stats().attack_damage > 0.0);
        assert!(BuildingType::LLT.stats().attack_range > 0.0);
    }

    // --- Terrain noise ---

    #[test]
    fn hash_noise_deterministic() {
        let a = hash_noise(3.14, 2.71);
        let b = hash_noise(3.14, 2.71);
        assert_eq!(a, b);
    }

    #[test]
    fn hash_noise_varies() {
        let a = hash_noise(0.0, 0.0);
        let b = hash_noise(1.0, 0.0);
        let c = hash_noise(0.0, 1.0);
        // At least two should differ (very unlikely all same)
        assert!(a != b || b != c, "hash_noise should vary with inputs");
    }

    #[test]
    fn hash_noise_bounded() {
        // hash_noise uses .fract().abs() so should be in [0, 1)
        for i in 0..100 {
            let x = i as f32 * 0.37;
            let y = i as f32 * 0.59;
            let v = hash_noise(x, y);
            assert!(v >= 0.0 && v <= 1.0, "hash_noise({}, {}) = {} out of [0,1]", x, y, v);
        }
    }

    #[test]
    fn terrain_noise_deterministic() {
        let a = terrain_noise(5.0, 10.0);
        let b = terrain_noise(5.0, 10.0);
        assert_eq!(a, b);
    }

    #[test]
    fn terrain_noise_bounded() {
        for i in 0..100 {
            let x = i as f32 * 0.5;
            let y = i as f32 * 0.3;
            let v = terrain_noise(x, y);
            assert!(v >= 0.0 && v <= 1.0, "terrain_noise({}, {}) = {} out of [0,1]", x, y, v);
        }
    }

    // --- Terrain heightmap additional ---

    #[test]
    fn terrain_height_bilinear_continuous() {
        let terrain = TerrainHeightmap::generate();
        // Walk in small steps; consecutive samples should differ by a bounded amount
        let max_jump = 5.0; // max height change over 1 game unit
        let mut prev = terrain.height_at(0.0, 0.0);
        for i in 1..(MAP_SIZE as usize) {
            let x = i as f32;
            let h = terrain.height_at(x, x);
            assert!(
                (h - prev).abs() < max_jump,
                "height jump too large at ({0},{0}): {1} -> {2}",
                x, prev, h,
            );
            prev = h;
        }
    }

    #[test]
    fn terrain_ridges_are_higher_than_valley() {
        let terrain = TerrainHeightmap::generate();
        // NW ridge center should be higher than the valley floor center
        let ridge_h = terrain.height_at(450.0, 450.0);
        let valley_h = terrain.height_at(700.0, 200.0);
        assert!(
            ridge_h > valley_h,
            "NW ridge ({}) should be higher than valley ({})",
            ridge_h, valley_h,
        );
    }

    #[test]
    fn terrain_edges_are_flat() {
        let terrain = TerrainHeightmap::generate();
        // Corners should be very low due to edge falloff
        let corner_h = terrain.height_at(0.0, 0.0);
        assert!(
            corner_h < 2.0,
            "map corner should be near 0 due to edge falloff, got {}",
            corner_h,
        );
    }

    #[test]
    fn terrain_spawn_areas_elevated() {
        let terrain = TerrainHeightmap::generate();
        // Spawn areas at (200,200) and (1800,1800) should be on plateaus (~12)
        let player_h = terrain.height_at(200.0, 200.0);
        let enemy_h = terrain.height_at(1800.0, 1800.0);
        assert!(player_h > 8.0, "player spawn height {} too low", player_h);
        assert!(enemy_h > 8.0, "enemy spawn height {} too low", enemy_h);
    }

    // --- NavGrid additional ---

    #[test]
    fn navgrid_cell_to_game_within_map() {
        let terrain = TerrainHeightmap::generate();
        let grid = NavGrid::new(&terrain);
        for cy in 0..NAV_GRID_SIZE {
            for cx in 0..NAV_GRID_SIZE {
                let pos = grid.cell_to_game(cx, cy);
                assert!(pos.x >= 0.0 && pos.x <= MAP_SIZE, "cell ({},{}) x={}", cx, cy, pos.x);
                assert!(pos.y >= 0.0 && pos.y <= MAP_SIZE, "cell ({},{}) y={}", cx, cy, pos.y);
            }
        }
    }

    #[test]
    fn navgrid_game_to_cell_clamps() {
        let terrain = TerrainHeightmap::generate();
        let grid = NavGrid::new(&terrain);
        // Large coords should clamp to last cell
        let (cx, cy) = grid.game_to_cell(99999.0, 99999.0);
        assert!(cx < NAV_GRID_SIZE);
        assert!(cy < NAV_GRID_SIZE);
    }

    #[test]
    fn navgrid_mark_multiple_buildings_independent() {
        let terrain = TerrainHeightmap::generate();
        let mut grid = NavGrid::new(&terrain);
        let pos_a = Vec2::new(200.0, 200.0);
        let pos_b = Vec2::new(800.0, 800.0);
        grid.mark_building(pos_a, (32.0, 32.0), true);
        grid.mark_building(pos_b, (32.0, 32.0), true);

        let (cx_a, cy_a) = grid.game_to_cell(200.0, 200.0);
        let (cx_b, cy_b) = grid.game_to_cell(800.0, 800.0);
        assert!(grid.blocked[cy_a * NAV_GRID_SIZE + cx_a]);
        assert!(grid.blocked[cy_b * NAV_GRID_SIZE + cx_b]);

        // Unmark A, B should still be blocked
        grid.mark_building(pos_a, (32.0, 32.0), false);
        assert!(!grid.blocked[cy_a * NAV_GRID_SIZE + cx_a]);
        assert!(grid.blocked[cy_b * NAV_GRID_SIZE + cx_b]);
    }

    #[test]
    fn navgrid_blocked_cell_not_passable() {
        let terrain = TerrainHeightmap::generate();
        let mut grid = NavGrid::new(&terrain);
        let (cx, cy) = grid.game_to_cell(200.0, 200.0);
        // Should start passable
        assert!(grid.is_passable(cx, cy, MoveClass::Bot));
        // Block it
        grid.blocked[cy * NAV_GRID_SIZE + cx] = true;
        assert!(!grid.is_passable(cx, cy, MoveClass::Bot));
        assert!(!grid.is_passable(cx, cy, MoveClass::Vehicle));
    }

    // --- snap_to_build_grid additional ---

    #[test]
    fn snap_to_grid_near_zero() {
        let snapped = snap_to_build_grid(Vec2::new(1.0, 1.0), (32.0, 32.0));
        assert!(snapped.x >= 0.0);
        assert!(snapped.y >= 0.0);
    }

    #[test]
    fn snap_to_grid_near_map_edge() {
        let snapped = snap_to_build_grid(Vec2::new(MAP_SIZE - 1.0, MAP_SIZE - 1.0), (32.0, 32.0));
        // Should snap to a valid position near the edge
        assert!(snapped.x > MAP_SIZE - BUILD_GRID_SIZE * 2.0);
        assert!(snapped.y > MAP_SIZE - BUILD_GRID_SIZE * 2.0);
    }

    #[test]
    fn snap_all_building_sizes_idempotent() {
        let types = [
            BuildingType::MetalExtractor,
            BuildingType::SolarCollector,
            BuildingType::Factory,
            BuildingType::LLT,
            BuildingType::Wall,
            BuildingType::RadarTower,
        ];
        for bt in &types {
            let size = bt.stats().size;
            let first = snap_to_build_grid(Vec2::new(500.0, 500.0), size);
            let second = snap_to_build_grid(first, size);
            assert!(
                (first.x - second.x).abs() < f32::EPSILON
                    && (first.y - second.y).abs() < f32::EPSILON,
                "{:?}: snap not idempotent (first={:?}, second={:?})",
                bt, first, second,
            );
        }
    }

    // --- Explosion particle determinism ---

    #[test]
    fn explosion_particle_spread_deterministic() {
        // Replicate the explosion angle computation from combat.rs
        let seed = 42u64;
        let mut angles = Vec::new();
        for i in 0..8u64 {
            let angle = (seed.wrapping_add(i).wrapping_mul(2654435761)) as f32
                / u32::MAX as f32
                * std::f32::consts::TAU;
            angles.push(angle);
        }
        // Run again with same seed
        let mut angles2 = Vec::new();
        for i in 0..8u64 {
            let angle = (seed.wrapping_add(i).wrapping_mul(2654435761)) as f32
                / u32::MAX as f32
                * std::f32::consts::TAU;
            angles2.push(angle);
        }
        assert_eq!(angles, angles2, "explosion spread must be deterministic");
    }

    #[test]
    fn explosion_particle_spread_distinct_angles() {
        let seed = 100u64;
        let mut angles = Vec::new();
        for i in 0..8u64 {
            let angle = (seed.wrapping_add(i).wrapping_mul(2654435761)) as f32
                / u32::MAX as f32
                * std::f32::consts::TAU;
            angles.push(angle);
        }
        // All 8 angles should be distinct
        for i in 0..angles.len() {
            for j in (i + 1)..angles.len() {
                assert!(
                    (angles[i] - angles[j]).abs() > 0.01,
                    "particles {} and {} have nearly identical angles",
                    i, j,
                );
            }
        }
    }

    #[test]
    fn explosion_particle_velocities_have_upward_component() {
        let seed = 7u64;
        for i in 0..8u64 {
            let angle = (seed.wrapping_add(i).wrapping_mul(2654435761)) as f32
                / u32::MAX as f32
                * std::f32::consts::TAU;
            let speed = 40.0 + (i as f32) * 8.0;
            let velocity = Vec3::new(
                angle.cos() * speed,
                30.0 + (i as f32) * 5.0,
                angle.sin() * speed,
            );
            assert!(
                velocity.y > 0.0,
                "particle {} should have upward velocity, got y={}",
                i, velocity.y,
            );
        }
    }

    #[test]
    fn explosion_particle_lifetimes_positive_and_increasing() {
        for i in 0..8u64 {
            let lifetime = 0.6 + (i as f32) * 0.05;
            assert!(lifetime > 0.0);
            if i > 0 {
                let prev = 0.6 + ((i - 1) as f32) * 0.05;
                assert!(lifetime > prev);
            }
        }
    }

    #[test]
    fn explosion_spread_different_seeds_differ() {
        let compute_angles = |seed: u64| -> Vec<f32> {
            (0..8u64)
                .map(|i| {
                    (seed.wrapping_add(i).wrapping_mul(2654435761)) as f32
                        / u32::MAX as f32
                        * std::f32::consts::TAU
                })
                .collect()
        };
        let a = compute_angles(1);
        let b = compute_angles(2);
        assert_ne!(a, b, "different seeds should produce different spreads");
    }

    // --- Minimap coordinate mapping ---

    #[test]
    fn minimap_coord_mapping_corners() {
        // Verify the minimap coordinate formula used in visuals.rs
        let check = |gx: f32, gy: f32| {
            let rel_x = (gx / MAP_SIZE).clamp(0.0, 1.0);
            let rel_y = (gy / MAP_SIZE).clamp(0.0, 1.0);
            let px = rel_x * MINIMAP_SIZE - 2.0;
            let py = rel_y * MINIMAP_SIZE - 2.0;
            (px, py)
        };
        // Bottom-left corner
        let (px, py) = check(0.0, 0.0);
        assert!((px - (-2.0)).abs() < f32::EPSILON);
        assert!((py - (-2.0)).abs() < f32::EPSILON);
        // Top-right corner
        let (px, py) = check(MAP_SIZE, MAP_SIZE);
        assert!((px - (MINIMAP_SIZE - 2.0)).abs() < f32::EPSILON);
        assert!((py - (MINIMAP_SIZE - 2.0)).abs() < f32::EPSILON);
        // Center
        let (px, py) = check(MAP_SIZE / 2.0, MAP_SIZE / 2.0);
        assert!((px - (MINIMAP_SIZE / 2.0 - 2.0)).abs() < f32::EPSILON);
        assert!((py - (MINIMAP_SIZE / 2.0 - 2.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn minimap_coord_clamps_out_of_bounds() {
        let gx = -100.0;
        let gy = MAP_SIZE + 500.0;
        let rel_x = (gx / MAP_SIZE).clamp(0.0, 1.0);
        let rel_y = (gy / MAP_SIZE).clamp(0.0, 1.0);
        assert_eq!(rel_x, 0.0);
        assert_eq!(rel_y, 1.0);
    }
}