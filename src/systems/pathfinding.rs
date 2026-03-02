use bevy::prelude::*;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

use crate::types::*;

// --- A* Pathfinding ---

/// Octile heuristic for 8-directional movement
fn octile_heuristic(ax: usize, ay: usize, bx: usize, by: usize) -> u32 {
    let dx = (ax as i32 - bx as i32).unsigned_abs();
    let dy = (ay as i32 - by as i32).unsigned_abs();
    let (min, max) = if dx < dy { (dx, dy) } else { (dy, dx) };
    // 10 = cardinal cost, 14 = diagonal cost (approx sqrt(2)*10)
    min * 14 + (max - min) * 10
}

#[derive(Eq, PartialEq)]
struct AStarNode {
    f_cost: u32,
    g_cost: u32,
    index: usize, // cy * NAV_GRID_SIZE + cx
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower f_cost first, then lower g_cost (prefer closer to goal), then by index for determinism
        other.f_cost.cmp(&self.f_cost)
            .then_with(|| other.g_cost.cmp(&self.g_cost))
            .then_with(|| other.index.cmp(&self.index))
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 8 directions: (dx, dy, cost) — cardinals cost 10, diagonals cost 14
const DIRS: [(i32, i32, u32); 8] = [
    (1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10),
    (1, 1, 14), (-1, 1, 14), (1, -1, 14), (-1, -1, 14),
];

/// Find a path from start to goal on the nav grid.
/// `stop_distance_cells` allows early termination when within N cells of goal.
/// Returns waypoints in game coordinates (start excluded, goal included).
pub fn find_path(
    nav_grid: &NavGrid,
    start: Vec2,
    goal: Vec2,
    move_class: MoveClass,
    stop_distance_cells: usize,
) -> Option<Vec<Vec2>> {
    let (sx, sy) = nav_grid.game_to_cell(start.x, start.y);
    let (gx, gy) = nav_grid.game_to_cell(goal.x, goal.y);

    let size = NAV_GRID_SIZE;
    let total = size * size;

    // If start == goal, no path needed
    if sx == gx && sy == gy {
        return Some(vec![goal]);
    }

    let start_idx = sy * size + sx;

    let mut g_cost = vec![u32::MAX; total];
    let mut came_from = vec![u32::MAX; total]; // parent index
    let mut closed = vec![false; total];

    g_cost[start_idx] = 0;

    let mut open = BinaryHeap::new();
    open.push(AStarNode {
        f_cost: octile_heuristic(sx, sy, gx, gy),
        g_cost: 0,
        index: start_idx,
    });

    let mut found_idx: Option<usize> = None;

    while let Some(current) = open.pop() {
        let ci = current.index;
        if closed[ci] {
            continue;
        }
        closed[ci] = true;

        let cx = ci % size;
        let cy = ci / size;

        // Check stop distance
        let dx_goal = (cx as i32 - gx as i32).unsigned_abs() as usize;
        let dy_goal = (cy as i32 - gy as i32).unsigned_abs() as usize;
        if dx_goal <= stop_distance_cells && dy_goal <= stop_distance_cells {
            found_idx = Some(ci);
            break;
        }

        for &(ddx, ddy, step_cost) in &DIRS {
            let nx = cx as i32 + ddx;
            let ny = cy as i32 + ddy;

            if nx < 0 || nx >= size as i32 || ny < 0 || ny >= size as i32 {
                continue;
            }

            let nux = nx as usize;
            let nuy = ny as usize;
            let ni = nuy * size + nux;

            if closed[ni] {
                continue;
            }

            if !nav_grid.is_passable(nux, nuy, move_class) {
                continue;
            }

            // Diagonal corner-cutting prevention: both adjacent cardinals must be passable
            if ddx != 0 && ddy != 0 {
                if !nav_grid.is_passable(cx.wrapping_add_signed(ddx as isize), cy, move_class)
                    || !nav_grid.is_passable(cx, cy.wrapping_add_signed(ddy as isize), move_class)
                {
                    continue;
                }
            }

            let new_g = current.g_cost + step_cost;
            if new_g < g_cost[ni] {
                g_cost[ni] = new_g;
                came_from[ni] = ci as u32;
                let h = octile_heuristic(nux, nuy, gx, gy);
                open.push(AStarNode {
                    f_cost: new_g + h,
                    g_cost: new_g,
                    index: ni,
                });
            }
        }
    }

    let end_idx = found_idx?;

    // Reconstruct path
    let mut path_indices = Vec::new();
    let mut current = end_idx;
    while current != start_idx {
        path_indices.push(current);
        let parent = came_from[current] as usize;
        if parent == u32::MAX as usize || parent == current {
            return None; // broken path
        }
        current = parent;
    }
    path_indices.reverse();

    // Convert to game coordinates, using goal for the final waypoint
    let waypoints: Vec<Vec2> = path_indices
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            if i == path_indices.len() - 1 {
                // Use exact goal position for the last waypoint
                goal
            } else {
                let cx = idx % size;
                let cy = idx / size;
                nav_grid.cell_to_game(cx, cy)
            }
        })
        .collect();

    if waypoints.is_empty() {
        Some(vec![goal])
    } else {
        Some(waypoints)
    }
}

// --- Systems ---

/// Sync the NavGrid blocked state from buildings each tick.
pub fn navgrid_sync_system(
    mut nav_grid: ResMut<NavGrid>,
    buildings: Query<(&Transform, &Building)>,
) {
    // Clear blocked
    for cell in nav_grid.blocked.iter_mut() {
        *cell = false;
    }

    // Mark all built buildings
    for (tf, building) in &buildings {
        if !building.built {
            continue;
        }
        let pos = game_xy(&tf.translation);
        let size = building.building_type.stats().size;
        nav_grid.mark_building(pos, size, true);
    }

    nav_grid.finish_sync();
}

/// Compute stop distance in cells from the desired game-unit stop distance
fn stop_distance_cells(game_units: f32) -> usize {
    (game_units / BUILD_GRID_SIZE).ceil() as usize
}

/// Pathfinding system: compute or revalidate paths for units with movement targets.
pub fn pathfinding_system(
    mut commands: Commands,
    nav_grid: Res<NavGrid>,
    mut units: Query<(
        Entity,
        &Transform,
        &Unit,
        &MoveClass,
        Option<&mut Path>,
        Option<&MoveTarget>,
        Option<&AttackTarget>,
        Option<&BuildTarget>,
        Option<&ReclaimTarget>,
    )>,
    target_positions: Query<&Transform, Without<MoveClass>>,
) {
    for (entity, transform, unit, move_class, path_opt, move_target, attack_target, build_target, reclaim_target) in &mut units {
        if unit.speed == 0.0 {
            continue;
        }

        let pos = game_xy(&transform.translation);

        // Determine goal and stop distance based on target type (priority: BuildTarget > AttackTarget > ReclaimTarget > MoveTarget)
        let (goal, stop_dist) = if let Some(BuildTarget(target_entity)) = build_target {
            if let Ok(target_tf) = target_positions.get(*target_entity) {
                (game_xy(&target_tf.translation), BUILD_RANGE * 0.9)
            } else {
                continue;
            }
        } else if let Some(AttackTarget(target_entity)) = attack_target {
            if let Ok(target_tf) = target_positions.get(*target_entity) {
                (game_xy(&target_tf.translation), unit.attack_range * 0.9)
            } else {
                continue;
            }
        } else if let Some(ReclaimTarget(target_entity)) = reclaim_target {
            if let Ok(target_tf) = target_positions.get(*target_entity) {
                (game_xy(&target_tf.translation), RECLAIM_RANGE)
            } else {
                continue;
            }
        } else if let Some(MoveTarget(target)) = move_target {
            (*target, 0.0)
        } else {
            // No target — remove stale path
            if path_opt.is_some() {
                commands.entity(entity).remove::<Path>();
            }
            continue;
        };

        // Check if already within stop distance
        if pos.distance(goal) <= stop_dist + BUILD_GRID_SIZE {
            if path_opt.is_some() {
                commands.entity(entity).remove::<Path>();
            }
            continue;
        }

        let stop_cells = stop_distance_cells(stop_dist);

        // Check if existing path is still valid
        if let Some(mut path) = path_opt {
            // Off-course check: if unit is >3 cells from next waypoint, force recompute
            let off_course = if let Some(next_wp) = path.waypoints.first() {
                pos.distance(*next_wp) > BUILD_GRID_SIZE * 3.0
            } else {
                false
            };

            // Goal drift check: if target moved significantly, recompute
            if off_course || path.goal.distance(goal) > BUILD_GRID_SIZE * 2.0 {
                // Recompute below
            } else if path.grid_version == nav_grid.version {
                // Path is current, keep it
                continue;
            } else {
                // Grid changed — quick validation: check all waypoint cells still passable
                let all_clear = path.waypoints.iter().all(|wp| {
                    let (cx, cy) = nav_grid.game_to_cell(wp.x, wp.y);
                    nav_grid.is_passable(cx, cy, *move_class)
                });
                if all_clear {
                    path.grid_version = nav_grid.version;
                    continue;
                }
                // Fall through to recompute
            }
        }

        // Compute new path
        if let Some(waypoints) = find_path(&nav_grid, pos, goal, *move_class, stop_cells) {
            commands.entity(entity).insert(Path {
                waypoints,
                grid_version: nav_grid.version,
                goal,
                stuck_timer: 0.0,
                last_dist_to_wp: f32::MAX,
            });
        } else {
            // No path found — remove Path so unit falls back to beeline
            commands.entity(entity).remove::<Path>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_grid() -> NavGrid {
        let terrain = TerrainHeightmap::generate();
        NavGrid::new(&terrain)
    }

    #[test]
    fn find_path_straight_line() {
        let grid = make_test_grid();
        // Path across flat spawn area
        let start = Vec2::new(200.0, 200.0);
        let goal = Vec2::new(300.0, 200.0);
        let result = find_path(&grid, start, goal, MoveClass::Bot, 0);
        assert!(result.is_some());
        let waypoints = result.unwrap();
        assert!(!waypoints.is_empty());
        // Last waypoint should be the goal
        let last = waypoints.last().unwrap();
        assert_eq!(*last, goal);
    }

    #[test]
    fn find_path_around_obstacle() {
        let mut grid = make_test_grid();
        // Place a wall of blocked cells across the path
        for cy in 8..15 {
            let idx = cy * NAV_GRID_SIZE + 12;
            grid.blocked[idx] = true;
        }
        let start = Vec2::new(160.0, 180.0); // cell ~(10, 11)
        let goal = Vec2::new(240.0, 180.0);  // cell ~(15, 11)
        let result = find_path(&grid, start, goal, MoveClass::Bot, 0);
        assert!(result.is_some());
        let waypoints = result.unwrap();
        // Path should go around the wall (not through blocked cells)
        for wp in &waypoints {
            let (cx, cy) = grid.game_to_cell(wp.x, wp.y);
            assert!(grid.is_passable(cx, cy, MoveClass::Bot), "waypoint at blocked cell ({}, {})", cx, cy);
        }
    }

    #[test]
    fn find_path_same_cell() {
        let grid = make_test_grid();
        let pos = Vec2::new(200.0, 200.0);
        let result = find_path(&grid, pos, pos, MoveClass::Bot, 0);
        assert!(result.is_some());
    }

    #[test]
    fn find_path_with_stop_distance() {
        let grid = make_test_grid();
        let start = Vec2::new(200.0, 200.0);
        let goal = Vec2::new(400.0, 200.0);
        let stop_cells = 3;
        let result = find_path(&grid, start, goal, MoveClass::Bot, stop_cells);
        assert!(result.is_some());
        let waypoints = result.unwrap();
        // Last waypoint should be near goal but not necessarily at it
        let last = waypoints.last().unwrap();
        let (lx, ly) = grid.game_to_cell(last.x, last.y);
        let (gx, gy) = grid.game_to_cell(goal.x, goal.y);
        let dx = (lx as i32 - gx as i32).unsigned_abs() as usize;
        let dy = (ly as i32 - gy as i32).unsigned_abs() as usize;
        assert!(dx <= stop_cells && dy <= stop_cells);
    }

    #[test]
    fn find_path_no_path_when_fully_blocked() {
        let mut grid = make_test_grid();
        // Block a ring around the goal
        let (gx, gy) = grid.game_to_cell(400.0, 400.0);
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let cx = (gx as i32 + dx) as usize;
                let cy = (gy as i32 + dy) as usize;
                if cx < NAV_GRID_SIZE && cy < NAV_GRID_SIZE {
                    grid.blocked[cy * NAV_GRID_SIZE + cx] = true;
                }
            }
        }
        let start = Vec2::new(200.0, 200.0);
        let goal = Vec2::new(400.0, 400.0);
        let result = find_path(&grid, start, goal, MoveClass::Bot, 0);
        assert!(result.is_none());
    }

    #[test]
    fn find_path_deterministic() {
        let grid = make_test_grid();
        let start = Vec2::new(200.0, 200.0);
        let goal = Vec2::new(500.0, 500.0);
        let a = find_path(&grid, start, goal, MoveClass::Bot, 0);
        let b = find_path(&grid, start, goal, MoveClass::Bot, 0);
        assert_eq!(a, b, "pathfinding must be deterministic");
    }

    #[test]
    fn octile_heuristic_values() {
        // Straight horizontal: 5 cells → 50
        assert_eq!(octile_heuristic(0, 0, 5, 0), 50);
        // Straight vertical: 3 cells → 30
        assert_eq!(octile_heuristic(0, 0, 0, 3), 30);
        // Diagonal: 3,3 → 3*14 = 42
        assert_eq!(octile_heuristic(0, 0, 3, 3), 42);
        // Mixed: 5,3 → 3*14 + 2*10 = 42 + 20 = 62
        assert_eq!(octile_heuristic(0, 0, 5, 3), 62);
    }
}
