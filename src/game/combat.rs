use bevy::ecs::entity::EntityHashSet;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use smallvec::SmallVec;

use super::GameplaySet;
use super::effects::{self, ShakeEvent};
use super::enemy::{
    ENEMY_BULLET_SIZE, ENEMY_SCALE, ENEMY_SIZE, Enemy, EnemyBullet, EnemyHealth, EnemyHitFlash,
    EnemyType,
};
use super::player::{
    BULLET_SCALE, BULLET_SIZE, Bullet, Invincible, PLAYER_SCALE, PLAYER_SIZE, Player, PowerUpType,
    spawn_powerup,
};
use super::shared::{Collider, GameBounds, Health, Score};
use super::state::{GameState, PlayState};

const POWERUP_DROP_RATE: f32 = 0.3;
const R_BULLET_ENEMY: i32 = 2;
const R_PLAYER_ENEMY: i32 = 2;
const R_PLAYER_BULLET: i32 = 2;

type GridEntry = (Entity, Vec3, Vec2);

#[derive(Component)]
pub struct Pierce(pub u32);

#[derive(Component, Default)]
pub struct HitList {
    hit: [Option<Entity>; 3],
    len: u8,
}

impl HitList {
    pub fn contains(&self, entity: Entity) -> bool {
        self.hit[..self.len as usize].contains(&Some(entity))
    }

    pub fn push(&mut self, entity: Entity) {
        if (self.len as usize) < self.hit.len() {
            self.hit[self.len as usize] = Some(entity);
            self.len += 1;
        }
    }
}

#[derive(Default)]
struct SpatialGrid {
    cells: Vec<SmallVec<[GridEntry; 4]>>,
    touched: Vec<usize>,
    cols: usize,
    rows: usize,
    offset_x: f32,
    offset_y: f32,
}

impl SpatialGrid {
    const CELL_SIZE: f32 = 64.0;
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

    fn cell_coords(&self, position: Vec3) -> (i32, i32) {
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

    fn get_cell(&self, cx: i32, cy: i32) -> Option<&[(Entity, Vec3, Vec2)]> {
        if cx >= 0 && cy >= 0 && (cx as usize) < self.cols && (cy as usize) < self.rows {
            let index = (cy as usize) * self.cols + (cx as usize);
            Some(&self.cells[index])
        } else {
            None
        }
    }
}

#[derive(Resource, Default)]
struct CollisionCache {
    hit_bullets: EntityHashSet,
    hit_enemies: EntityHashSet,
    enemy_grid: SpatialGrid,
    enemy_bullet_grid: SpatialGrid,
    reserved: bool,
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

        if !self.reserved {
            self.hit_bullets.reserve(128);
            self.hit_enemies.reserve(64);
            self.reserved = true;
        }

        self.hit_bullets.clear();
        self.hit_enemies.clear();
    }
}

enum CombatOutcome {
    Despawn(Entity),
    EnemyHit(Entity),
    EnemyDestroyed {
        entity: Entity,
        position: Vec3,
        score: u32,
        drop: Option<PowerUpType>,
    },
    PlayerDamaged {
        player: Entity,
        defeated: bool,
        consumed: Entity,
    },
}

#[derive(Resource, Default)]
struct CombatFrame {
    outcomes: Vec<CombatOutcome>,
    player_damage_resolved: bool,
    shake_magnitude: f32,
    shake_duration: f32,
}

impl CombatFrame {
    fn clear(&mut self) {
        self.outcomes.clear();
        self.player_damage_resolved = false;
        self.shake_magnitude = 0.0;
        self.shake_duration = 0.0;
    }

    fn push(&mut self, outcome: CombatOutcome) {
        self.outcomes.push(outcome);
    }

    fn add_shake(&mut self, magnitude: f32, duration: f32) {
        self.shake_magnitude = self.shake_magnitude.max(magnitude);
        self.shake_duration = self.shake_duration.max(duration);
    }
}

type BulletCollisionQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static Collider,
        Option<&'static mut Pierce>,
        Option<&'static mut HitList>,
    ),
    With<Bullet>,
>;

type EnemyHealthQuery<'w, 's> =
    Query<'w, 's, (&'static mut EnemyHealth, &'static EnemyType), With<Enemy>>;

type PlayerBodyQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static Collider,
        &'static mut Health,
        Option<&'static Invincible>,
    ),
    With<Player>,
>;

#[derive(SystemParam)]
struct BulletEnemyQueries<'w, 's> {
    bullets: BulletCollisionQuery<'w, 's>,
    enemy_health: EnemyHealthQuery<'w, 's>,
}

#[derive(SystemParam)]
struct PlayerCollisionQuery<'w, 's> {
    player: PlayerBodyQuery<'w, 's>,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionCache>()
            .init_resource::<CombatFrame>()
            .add_systems(
                Update,
                (
                    prepare_collision_frame,
                    detect_bullet_enemy_collisions,
                    detect_player_enemy_collisions,
                    detect_player_enemy_bullet_collisions,
                )
                    .chain()
                    .in_set(GameplaySet::Collision)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            )
            .add_systems(
                Update,
                (apply_combat_outcomes, despawn_expired)
                    .chain()
                    .in_set(GameplaySet::Cleanup)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

pub fn collide(pos_a: Vec3, size_a: Vec2, pos_b: Vec3, size_b: Vec2) -> bool {
    let a_min = pos_a.truncate() - size_a / 2.0;
    let a_max = pos_a.truncate() + size_a / 2.0;
    let b_min = pos_b.truncate() - size_b / 2.0;
    let b_max = pos_b.truncate() + size_b / 2.0;

    a_min.x < b_max.x && a_max.x > b_min.x && a_min.y < b_max.y && a_max.y > b_min.y
}

fn prepare_collision_frame(
    mut cache: ResMut<CollisionCache>,
    mut frame: ResMut<CombatFrame>,
    bounds: Res<GameBounds>,
    enemies: Query<(Entity, &Transform, &Collider), With<Enemy>>,
    enemy_bullets: Query<(Entity, &Transform, &Collider), With<EnemyBullet>>,
) {
    frame.clear();
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

fn detect_bullet_enemy_collisions(
    mut cache: ResMut<CollisionCache>,
    mut frame: ResMut<CombatFrame>,
    mut queries: BulletEnemyQueries,
) {
    debug_assert_collision_radii();

    'bullet_loop: for (
        bullet_entity,
        bullet_transform,
        bullet_collider,
        mut pierce,
        mut hit_list,
    ) in &mut queries.bullets
    {
        if cache.hit_bullets.contains(&bullet_entity) {
            continue;
        }

        let bullet_position = bullet_transform.translation;
        let bullet_size = bullet_collider.size;
        let (cx, cy) = cache.enemy_grid.cell_coords(bullet_position);

        for x in (cx - R_BULLET_ENEMY)..=(cx + R_BULLET_ENEMY) {
            for y in (cy - R_BULLET_ENEMY)..=(cy + R_BULLET_ENEMY) {
                let Some(entries) = cache.enemy_grid.get_cell(x, y) else {
                    continue;
                };

                for &(enemy_entity, enemy_position, enemy_size) in entries {
                    if cache.hit_enemies.contains(&enemy_entity) {
                        continue;
                    }

                    if let Some(existing_hits) = hit_list.as_ref()
                        && existing_hits.contains(enemy_entity)
                    {
                        continue;
                    }

                    if !collide(bullet_position, bullet_size, enemy_position, enemy_size) {
                        continue;
                    }

                    if let Ok((mut enemy_health, enemy_type)) =
                        queries.enemy_health.get_mut(enemy_entity)
                    {
                        enemy_health.current = enemy_health.current.saturating_sub(1);

                        if enemy_health.current == 0 {
                            cache.hit_enemies.insert(enemy_entity);
                            frame.push(CombatOutcome::EnemyDestroyed {
                                entity: enemy_entity,
                                position: enemy_position,
                                score: enemy_type.score(),
                                drop: roll_powerup_drop(),
                            });
                            frame.add_shake(4.0, 0.06);
                        } else {
                            frame.push(CombatOutcome::EnemyHit(enemy_entity));
                            frame.add_shake(2.0, 0.03);
                        }

                        if let Some(existing_hits) = hit_list.as_mut() {
                            existing_hits.push(enemy_entity);
                        }

                        if let Some(pierce) = pierce.as_mut()
                            && pierce.0 > 0
                        {
                            pierce.0 -= 1;
                            continue 'bullet_loop;
                        }

                        cache.hit_bullets.insert(bullet_entity);
                        frame.push(CombatOutcome::Despawn(bullet_entity));
                        continue 'bullet_loop;
                    }
                }
            }
        }
    }
}

fn detect_player_enemy_collisions(
    mut cache: ResMut<CollisionCache>,
    mut frame: ResMut<CombatFrame>,
    mut player_query: PlayerCollisionQuery,
) {
    let Ok((player_entity, player_transform, player_collider, mut health, invincible)) =
        player_query.player.single_mut()
    else {
        return;
    };

    let player_position = player_transform.translation;
    let player_size = player_collider.size;
    let (cx, cy) = cache.enemy_grid.cell_coords(player_position);

    if invincible.is_some() {
        let CollisionCache {
            enemy_grid,
            hit_enemies,
            ..
        } = &mut *cache;
        for x in (cx - R_PLAYER_ENEMY)..=(cx + R_PLAYER_ENEMY) {
            for y in (cy - R_PLAYER_ENEMY)..=(cy + R_PLAYER_ENEMY) {
                let Some(entries) = enemy_grid.get_cell(x, y) else {
                    continue;
                };

                for &(enemy_entity, enemy_position, enemy_size) in entries {
                    if hit_enemies.contains(&enemy_entity)
                        || !collide(player_position, player_size, enemy_position, enemy_size)
                    {
                        continue;
                    }

                    hit_enemies.insert(enemy_entity);
                    frame.push(CombatOutcome::EnemyDestroyed {
                        entity: enemy_entity,
                        position: enemy_position,
                        score: 5,
                        drop: None,
                    });
                    frame.add_shake(4.0, 0.06);
                }
            }
        }

        return;
    }

    let CollisionCache {
        enemy_grid,
        hit_enemies,
        ..
    } = &mut *cache;
    for x in (cx - R_PLAYER_ENEMY)..=(cx + R_PLAYER_ENEMY) {
        for y in (cy - R_PLAYER_ENEMY)..=(cy + R_PLAYER_ENEMY) {
            let Some(entries) = enemy_grid.get_cell(x, y) else {
                continue;
            };

            for &(enemy_entity, enemy_position, enemy_size) in entries {
                if hit_enemies.contains(&enemy_entity)
                    || !collide(player_position, player_size, enemy_position, enemy_size)
                {
                    continue;
                }

                health.current = health.current.saturating_sub(1);
                hit_enemies.insert(enemy_entity);
                frame.player_damage_resolved = true;
                frame.push(CombatOutcome::PlayerDamaged {
                    player: player_entity,
                    defeated: health.current == 0,
                    consumed: enemy_entity,
                });

                if enemy_position != Vec3::ZERO {
                    frame.add_shake(
                        if health.current == 0 { 20.0 } else { 14.0 },
                        if health.current == 0 { 0.3 } else { 0.15 },
                    );
                }
                return;
            }
        }
    }
}

fn detect_player_enemy_bullet_collisions(
    cache: Res<CollisionCache>,
    mut frame: ResMut<CombatFrame>,
    mut player_query: PlayerCollisionQuery,
) {
    let Ok((player_entity, player_transform, player_collider, mut health, invincible)) =
        player_query.player.single_mut()
    else {
        return;
    };

    let player_position = player_transform.translation;
    let player_size = player_collider.size;
    let (bx, by) = cache.enemy_bullet_grid.cell_coords(player_position);

    if invincible.is_some() {
        for x in (bx - R_PLAYER_BULLET)..=(bx + R_PLAYER_BULLET) {
            for y in (by - R_PLAYER_BULLET)..=(by + R_PLAYER_BULLET) {
                let Some(entries) = cache.enemy_bullet_grid.get_cell(x, y) else {
                    continue;
                };

                for &(bullet_entity, bullet_position, bullet_size) in entries {
                    if collide(player_position, player_size, bullet_position, bullet_size) {
                        frame.push(CombatOutcome::Despawn(bullet_entity));
                    }
                }
            }
        }

        return;
    }

    if frame.player_damage_resolved {
        return;
    }

    for x in (bx - R_PLAYER_BULLET)..=(bx + R_PLAYER_BULLET) {
        for y in (by - R_PLAYER_BULLET)..=(by + R_PLAYER_BULLET) {
            let Some(entries) = cache.enemy_bullet_grid.get_cell(x, y) else {
                continue;
            };

            for &(bullet_entity, bullet_position, bullet_size) in entries {
                if !collide(player_position, player_size, bullet_position, bullet_size) {
                    continue;
                }

                health.current = health.current.saturating_sub(1);
                frame.player_damage_resolved = true;
                frame.push(CombatOutcome::PlayerDamaged {
                    player: player_entity,
                    defeated: health.current == 0,
                    consumed: bullet_entity,
                });
                frame.add_shake(
                    if health.current == 0 { 20.0 } else { 14.0 },
                    if health.current == 0 { 0.3 } else { 0.15 },
                );
                return;
            }
        }
    }
}

fn apply_combat_outcomes(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut next_state: ResMut<NextState<GameState>>,
    mut frame: ResMut<CombatFrame>,
    mut shake: MessageWriter<ShakeEvent>,
) {
    for outcome in frame.outcomes.drain(..) {
        match outcome {
            CombatOutcome::Despawn(entity) => {
                commands.entity(entity).despawn();
            }
            CombatOutcome::EnemyHit(entity) => {
                commands.entity(entity).insert(EnemyHitFlash::new());
            }
            CombatOutcome::EnemyDestroyed {
                entity,
                position,
                score: points,
                drop,
            } => {
                commands.entity(entity).despawn();
                effects::spawn_explosion(&mut commands, position);
                score.0 += points;

                if let Some(power_up) = drop {
                    spawn_powerup(&mut commands, position, power_up);
                }
            }
            CombatOutcome::PlayerDamaged {
                player,
                defeated,
                consumed,
            } => {
                commands.entity(consumed).despawn();
                if defeated {
                    next_state.set(GameState::GameOver);
                } else {
                    commands.entity(player).insert(Invincible::new());
                }
            }
        }
    }

    if frame.shake_magnitude > 0.0 && frame.shake_duration > 0.0 {
        shake.write(ShakeEvent {
            magnitude: frame.shake_magnitude,
            duration: frame.shake_duration,
        });
    }
}

fn despawn_expired(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut super::shared::Lifetime)>,
) {
    for (entity, mut lifetime) in &mut query {
        lifetime.0.tick(time.delta());
        if lifetime.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn roll_powerup_drop() -> Option<PowerUpType> {
    if fastrand::f32() >= POWERUP_DROP_RATE {
        return None;
    }

    Some(match fastrand::u32(0..4) {
        0 => PowerUpType::TripleShot,
        1 => PowerUpType::RapidFire,
        2 => PowerUpType::PierceShot,
        _ => PowerUpType::Shield,
    })
}

fn required_neighbor_radius(max_a: f32, scale_a: f32, max_b: f32, scale_b: f32) -> i32 {
    (((max_a * scale_a + max_b * scale_b) * 0.5) / SpatialGrid::CELL_SIZE).ceil() as i32 + 1
}

fn debug_assert_collision_radii() {
    #[cfg(debug_assertions)]
    {
        debug_assert!(
            R_BULLET_ENEMY
                >= required_neighbor_radius(
                    BULLET_SIZE.x.max(BULLET_SIZE.y),
                    BULLET_SCALE,
                    ENEMY_SIZE.x.max(ENEMY_SIZE.y),
                    ENEMY_SCALE,
                )
        );
        debug_assert!(
            R_PLAYER_ENEMY
                >= required_neighbor_radius(
                    PLAYER_SIZE.x.max(PLAYER_SIZE.y),
                    PLAYER_SCALE,
                    ENEMY_SIZE.x.max(ENEMY_SIZE.y),
                    ENEMY_SCALE,
                )
        );
        debug_assert!(
            R_PLAYER_BULLET
                >= required_neighbor_radius(
                    PLAYER_SIZE.x.max(PLAYER_SIZE.y),
                    PLAYER_SCALE,
                    ENEMY_BULLET_SIZE.x.max(ENEMY_BULLET_SIZE.y),
                    1.0,
                )
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_list_prevents_duplicate_hits() {
        let entity = Entity::from_raw_u32(7).expect("nonzero raw entity");
        let mut hit_list = HitList::default();

        assert!(!hit_list.contains(entity));
        hit_list.push(entity);
        assert!(hit_list.contains(entity));
    }

    #[test]
    fn collision_neighbor_radii_cover_all_colliders() {
        assert!(
            R_BULLET_ENEMY
                >= required_neighbor_radius(
                    BULLET_SIZE.x.max(BULLET_SIZE.y),
                    BULLET_SCALE,
                    ENEMY_SIZE.x.max(ENEMY_SIZE.y),
                    ENEMY_SCALE,
                )
        );
        assert!(
            R_PLAYER_ENEMY
                >= required_neighbor_radius(
                    PLAYER_SIZE.x.max(PLAYER_SIZE.y),
                    PLAYER_SCALE,
                    ENEMY_SIZE.x.max(ENEMY_SIZE.y),
                    ENEMY_SCALE,
                )
        );
        assert!(
            R_PLAYER_BULLET
                >= required_neighbor_radius(
                    PLAYER_SIZE.x.max(PLAYER_SIZE.y),
                    PLAYER_SCALE,
                    ENEMY_BULLET_SIZE.x.max(ENEMY_BULLET_SIZE.y),
                    1.0,
                )
        );
    }
}
