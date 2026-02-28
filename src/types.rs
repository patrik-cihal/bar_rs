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
pub const EXTRACTOR_INCOME: f32 = 2.0;
pub const SOLAR_INCOME: f32 = 10.0;
pub const RADAR_RANGE: f32 = 600.0;

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
    metal_cost: 20.0, energy_cost: 15.0, build_time: 3.0,
    sight_range: 400.0, radius: 10.0,
    model_file: "armpeep", model_scale: 0.35,
};

const RAIDER_STATS: UnitStats = UnitStats {
    name: "Raider",
    hp: 120.0, speed: 160.0, attack_damage: 15.0, attack_range: 130.0,
    attack_cooldown: 0.6, min_attack_range: 0.0,
    metal_cost: 35.0, energy_cost: 20.0, build_time: 4.0,
    sight_range: 250.0, radius: 12.0,
    model_file: "armflash", model_scale: 0.35,
};

const TANK_STATS: UnitStats = UnitStats {
    name: "Tank",
    hp: 200.0, speed: 120.0, attack_damage: 20.0, attack_range: 200.0,
    attack_cooldown: 0.8, min_attack_range: 0.0,
    metal_cost: 50.0, energy_cost: 30.0, build_time: 5.0,
    sight_range: 250.0, radius: 14.0,
    model_file: "armstump", model_scale: 0.35,
};

const ASSAULT_STATS: UnitStats = UnitStats {
    name: "Assault",
    hp: 400.0, speed: 60.0, attack_damage: 40.0, attack_range: 150.0,
    attack_cooldown: 1.2, min_attack_range: 0.0,
    metal_cost: 100.0, energy_cost: 60.0, build_time: 7.0,
    sight_range: 250.0, radius: 16.0,
    model_file: "armbull", model_scale: 0.35,
};

const ARTILLERY_STATS: UnitStats = UnitStats {
    name: "Artillery",
    hp: 100.0, speed: 50.0, attack_damage: 60.0, attack_range: 400.0,
    attack_cooldown: 2.0, min_attack_range: 150.0,
    metal_cost: 80.0, energy_cost: 50.0, build_time: 6.0,
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
    metal_cost: 60.0, energy_cost: 0.0, build_time: 3.0,
    size: (32.0, 32.0), hp: 300.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armmex", model_scale: 0.5,
};

const SOLAR_STATS: BuildingStats = BuildingStats {
    metal_cost: 20.0, energy_cost: 0.0, build_time: 2.0,
    size: (32.0, 32.0), hp: 300.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armsolar", model_scale: 0.5,
};

const FACTORY_STATS: BuildingStats = BuildingStats {
    metal_cost: 200.0, energy_cost: 200.0, build_time: 8.0,
    size: (48.0, 48.0), hp: 300.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armlab", model_scale: 0.5,
};

const LLT_STATS: BuildingStats = BuildingStats {
    metal_cost: 60.0, energy_cost: 0.0, build_time: 4.0,
    size: (32.0, 32.0), hp: 400.0,
    attack_damage: 25.0, attack_range: 250.0, attack_cooldown: 0.6,
    sight_range: 200.0,
    model_file: "armllt", model_scale: 0.5,
};

const WALL_STATS: BuildingStats = BuildingStats {
    metal_cost: 5.0, energy_cost: 0.0, build_time: 1.0,
    size: (16.0, 16.0), hp: 500.0,
    attack_damage: 0.0, attack_range: 0.0, attack_cooldown: 999.0,
    sight_range: 200.0,
    model_file: "armdrag", model_scale: 0.5,
};

const RADAR_STATS: BuildingStats = BuildingStats {
    metal_cost: 50.0, energy_cost: 30.0, build_time: 5.0,
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

#[derive(Component)]
pub struct PlayerOwned;

#[derive(Component)]
pub struct EnemyOwned;

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

#[derive(Clone, Copy, PartialEq)]
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
pub struct GameOverText;

#[derive(Component)]
pub struct DeathExplosion {
    pub timer: f32,
    pub max_radius: f32,
}

#[derive(Component)]
pub struct MinimapFrame;

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
            energy: 1000.0,
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
pub struct ModelLibrary {
    pub models: HashMap<String, Handle<Scene>>,
}

impl ModelLibrary {
    pub fn get(&self, model_file: &str, is_player: bool) -> Option<&Handle<Scene>> {
        let key = format!("{}_{}", model_file, if is_player { "blue" } else { "red" });
        self.models.get(&key)
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
pub const TERRAIN_MAX_HEIGHT: f32 = 30.0; // maximum hill height

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

    for _ in 0..3 {
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

        for gy in 0..TERRAIN_GRID_SIZE {
            for gx in 0..TERRAIN_GRID_SIZE {
                let wx = gx as f32 * cell_size;
                let wy = gy as f32 * cell_size;

                // Base terrain noise (scale controls feature size)
                let noise_scale = 0.008;
                let h = terrain_noise(wx * noise_scale + 3.7, wy * noise_scale + 7.1);

                // Flatten near map edges
                let edge_margin = 100.0;
                let ex = ((wx.min(MAP_SIZE - wx)) / edge_margin).min(1.0);
                let ey = ((wy.min(MAP_SIZE - wy)) / edge_margin).min(1.0);
                let edge_factor = ex * ey;

                // Flatten near player start (200,200) and enemy start (1800,1800)
                let player_dist = ((wx - 200.0).powi(2) + (wy - 200.0).powi(2)).sqrt();
                let enemy_dist = ((wx - 1800.0).powi(2) + (wy - 1800.0).powi(2)).sqrt();
                let start_radius = 250.0;
                let player_flat = (player_dist / start_radius).min(1.0).powi(2);
                let enemy_flat = (enemy_dist / start_radius).min(1.0).powi(2);
                let start_factor = player_flat.min(1.0) * enemy_flat.min(1.0);

                heights[gy * TERRAIN_GRID_SIZE + gx] = h * TERRAIN_MAX_HEIGHT * edge_factor * start_factor;
            }
        }

        // Helper: flatten a circular area around (px, py) with given radius
        let flatten_area = |heights: &mut Vec<f32>, px: f32, py: f32, radius: f32| {
            let gx_center = (px / cell_size).round() as usize;
            let gy_center = (py / cell_size).round() as usize;
            let gx_center = gx_center.min(TERRAIN_GRID_SIZE - 1);
            let gy_center = gy_center.min(TERRAIN_GRID_SIZE - 1);
            let plateau_height = heights[gy_center * TERRAIN_GRID_SIZE + gx_center];

            let grid_radius = (radius / cell_size).ceil() as i32 + 1;
            let gx_i = gx_center as i32;
            let gy_i = gy_center as i32;
            for dy in -grid_radius..=grid_radius {
                for dx in -grid_radius..=grid_radius {
                    let gx = (gx_i + dx) as usize;
                    let gy = (gy_i + dy) as usize;
                    if gx >= TERRAIN_GRID_SIZE || gy >= TERRAIN_GRID_SIZE {
                        continue;
                    }
                    let wx = gx as f32 * cell_size;
                    let wy = gy as f32 * cell_size;
                    let dist = ((wx - px).powi(2) + (wy - py).powi(2)).sqrt();
                    if dist < radius {
                        let t = (dist / radius).clamp(0.0, 1.0);
                        let blend = t * t * (3.0 - 2.0 * t); // smoothstep
                        let idx = gy * TERRAIN_GRID_SIZE + gx;
                        heights[idx] = plateau_height * (1.0 - blend) + heights[idx] * blend;
                    }
                }
            }
        };

        // Flatten around every metal spot so extractors are always placeable
        let metal_spot_positions = [
            (300.0, 300.0), (500.0, 200.0), (200.0, 600.0), (700.0, 400.0),
            (1000.0, 1000.0), (1300.0, 1600.0), (1500.0, 1400.0),
            (1700.0, 1800.0), (1800.0, 1500.0), (1600.0, 1700.0),
        ];
        for (mx, my) in metal_spot_positions {
            flatten_area(&mut heights, mx, my, 40.0);
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
