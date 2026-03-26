use bevy::prelude::*;
use smallvec::SmallVec;

use super::super::core::{Collider, GameBounds};
use super::super::enemy::{Enemy, EnemyBullet};

type GridEntry = (Entity, Vec3, Vec2);

#[derive(Default)]
pub(super) struct SpatialGrid {
    cells: Vec<SmallVec<[GridEntry; 4]>>,
    touched: Vec<usize>,
    cols: usize,
    rows: usize,
    offset_x: f32,
    offset_y: f32,
}

impl SpatialGrid {
    pub(super) const CELL_SIZE: f32 = 64.0;
    const INV_CELL_SIZE: f32 = 1.0 / 64.0;
    const MARGIN: f32 = 100.0;

    fn rebuild(&mut self, half_width: f32, half_height: f32) {
        let total_width = (half_width + Self::MARGIN) * 2.0;
        let total_height = (half_height + Self::MARGIN) * 2.0;

        self.cols = (total_width * Self::INV_CELL_SIZE).ceil() as usize + 1;
        self.rows = (total_height * Self::INV_CELL_SIZE).ceil() as usize + 1;
        self.offset_x = half_width + Self::MARGIN;
        self.offset_y = half_height + Self::MARGIN;

        let total_cells = self.cols * self.rows;
        if self.cells.len() < total_cells {
            self.cells.resize_with(total_cells, SmallVec::new);
        }

        for cell in &mut self.cells {
            cell.clear();
        }

        self.touched.clear();
        self.touched.reserve(64);
    }

    fn clear(&mut self) {
        for &index in &self.touched {
            if index < self.cells.len() {
                self.cells[index].clear();
            }
        }

        self.touched.clear();
    }

    fn cell_index(&self, position: Vec3) -> Option<usize> {
        let cx = ((position.x + self.offset_x) * Self::INV_CELL_SIZE).floor() as isize;
        let cy = ((position.y + self.offset_y) * Self::INV_CELL_SIZE).floor() as isize;

        if cx >= 0 && cy >= 0 && (cx as usize) < self.cols && (cy as usize) < self.rows {
            Some((cy as usize) * self.cols + (cx as usize))
        } else {
            None
        }
    }

    pub fn cell_coords(&self, position: Vec3) -> (i32, i32) {
        (
            ((position.x + self.offset_x) * Self::INV_CELL_SIZE).floor() as i32,
            ((position.y + self.offset_y) * Self::INV_CELL_SIZE).floor() as i32,
        )
    }

    fn insert_center(&mut self, entity: Entity, position: Vec3, size: Vec2) {
        if let Some(index) = self.cell_index(position) {
            let cell = &mut self.cells[index];
            if cell.is_empty() {
                self.touched.push(index);
            }
            cell.push((entity, position, size));
        }
    }

    pub fn get_cell(&self, cx: i32, cy: i32) -> Option<&[(Entity, Vec3, Vec2)]> {
        if cx >= 0 && cy >= 0 && (cx as usize) < self.cols && (cy as usize) < self.rows {
            let index = (cy as usize) * self.cols + (cx as usize);
            Some(&self.cells[index])
        } else {
            None
        }
    }
}

#[derive(Resource, Default)]
pub struct CollisionCache {
    enemy_grid: SpatialGrid,
    enemy_bullet_grid: SpatialGrid,
    last_half_width: f32,
    last_half_height: f32,
}

impl CollisionCache {
    fn prepare(&mut self, bounds: &GameBounds) {
        let resized = (self.last_half_width - bounds.half_width).abs() > 0.5
            || (self.last_half_height - bounds.half_height).abs() > 0.5;

        if self.enemy_grid.cols == 0 || resized {
            self.enemy_grid
                .rebuild(bounds.half_width, bounds.half_height);
            self.enemy_bullet_grid
                .rebuild(bounds.half_width, bounds.half_height);
            self.last_half_width = bounds.half_width;
            self.last_half_height = bounds.half_height;
        } else {
            self.enemy_grid.clear();
            self.enemy_bullet_grid.clear();
        }
    }

    pub fn enemy_grid(&self) -> &SpatialGrid {
        &self.enemy_grid
    }

    pub fn enemy_bullet_grid(&self) -> &SpatialGrid {
        &self.enemy_bullet_grid
    }
}

pub fn prepare_collision_cache(
    mut cache: ResMut<CollisionCache>,
    bounds: Res<GameBounds>,
    enemies: Query<(Entity, &Transform, &Collider), With<Enemy>>,
    enemy_bullets: Query<(Entity, &Transform, &Collider), With<EnemyBullet>>,
) {
    cache.prepare(&bounds);

    for (entity, transform, collider) in &enemies {
        cache
            .enemy_grid
            .insert_center(entity, transform.translation, collider.size);
    }

    for (entity, transform, collider) in &enemy_bullets {
        cache
            .enemy_bullet_grid
            .insert_center(entity, transform.translation, collider.size);
    }
}
