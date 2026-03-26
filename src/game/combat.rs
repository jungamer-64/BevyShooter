use bevy::ecs::entity::EntityHashSet;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use smallvec::SmallVec;

use super::core::{Collider, GameBounds, Health, Score};
use super::effects::{self, ShakeEvent};
use super::enemy::{
    ENEMY_BULLET_SIZE, ENEMY_SCALE, ENEMY_SIZE, Enemy, EnemyBullet, EnemyHitFlash, EnemyType,
};
use super::player::{
    BULLET_SCALE, BULLET_SIZE, Bullet, INVINCIBILITY_SECONDS, PLAYER_SCALE, PLAYER_SIZE, Player,
    PlayerStatus,
};
use super::powerup::{self, PowerUpKind};
use super::state::{GameState, PlayState};
use super::{GameplaySet, ResolveSet};

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

#[derive(Message, Debug, Clone, Copy)]
pub enum CombatMessage {
    Despawn(Entity),
    EnemyHit(Entity),
    EnemyDestroyed {
        entity: Entity,
        position: Vec3,
        score: u32,
        drop: Option<PowerUpKind>,
    },
    PlayerDamaged {
        player: Entity,
        defeated: bool,
        consumed: Entity,
    },
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
    Query<'w, 's, (&'static mut Health, &'static EnemyType), With<Enemy>>;

type PlayerBodyQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static Collider,
        &'static mut Health,
        &'static PlayerStatus,
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
            .add_message::<CombatMessage>()
            .add_systems(
                Update,
                (
                    prepare_collision_cache,
                    detect_bullet_enemy_collisions,
                    detect_player_collisions,
                )
                    .chain()
                    .in_set(GameplaySet::Detect)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            )
            .add_systems(
                Update,
                apply_combat_outcomes
                    .in_set(ResolveSet::Apply)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

fn prepare_collision_cache(
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

fn detect_bullet_enemy_collisions(
    mut cache: ResMut<CollisionCache>,
    mut combat: MessageWriter<CombatMessage>,
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

                    if !bullet_collider.intersects(
                        bullet_position,
                        Collider { size: enemy_size },
                        enemy_position,
                    ) {
                        continue;
                    }

                    if let Ok((mut enemy_health, enemy_type)) =
                        queries.enemy_health.get_mut(enemy_entity)
                    {
                        let destroyed = enemy_health.damage(1);

                        if destroyed {
                            cache.hit_enemies.insert(enemy_entity);
                            combat.write(CombatMessage::EnemyDestroyed {
                                entity: enemy_entity,
                                position: enemy_position,
                                score: enemy_type.score(),
                                drop: powerup::roll_drop(),
                            });
                        } else {
                            combat.write(CombatMessage::EnemyHit(enemy_entity));
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
                        combat.write(CombatMessage::Despawn(bullet_entity));
                        continue 'bullet_loop;
                    }
                }
            }
        }
    }
}

fn detect_player_collisions(
    mut cache: ResMut<CollisionCache>,
    mut combat: MessageWriter<CombatMessage>,
    mut player_query: PlayerCollisionQuery,
) {
    let Ok((player_entity, player_transform, player_collider, mut health, status)) =
        player_query.player.single_mut()
    else {
        return;
    };

    let player_position = player_transform.translation;
    let (enemy_x, enemy_y) = cache.enemy_grid.cell_coords(player_position);
    let (bullet_x, bullet_y) = cache.enemy_bullet_grid.cell_coords(player_position);

    if status.is_invincible() {
        {
            let CollisionCache {
                enemy_grid,
                hit_enemies,
                ..
            } = &mut *cache;

            for x in (enemy_x - R_PLAYER_ENEMY)..=(enemy_x + R_PLAYER_ENEMY) {
                for y in (enemy_y - R_PLAYER_ENEMY)..=(enemy_y + R_PLAYER_ENEMY) {
                    let Some(entries) = enemy_grid.get_cell(x, y) else {
                        continue;
                    };

                    for &(enemy_entity, enemy_position, enemy_size) in entries {
                        if hit_enemies.contains(&enemy_entity)
                            || !player_collider.intersects(
                                player_position,
                                Collider { size: enemy_size },
                                enemy_position,
                            )
                        {
                            continue;
                        }

                        hit_enemies.insert(enemy_entity);
                        combat.write(CombatMessage::EnemyDestroyed {
                            entity: enemy_entity,
                            position: enemy_position,
                            score: 5,
                            drop: None,
                        });
                    }
                }
            }
        }

        for x in (bullet_x - R_PLAYER_BULLET)..=(bullet_x + R_PLAYER_BULLET) {
            for y in (bullet_y - R_PLAYER_BULLET)..=(bullet_y + R_PLAYER_BULLET) {
                let Some(entries) = cache.enemy_bullet_grid.get_cell(x, y) else {
                    continue;
                };

                for &(bullet_entity, bullet_position, bullet_size) in entries {
                    if player_collider.intersects(
                        player_position,
                        Collider { size: bullet_size },
                        bullet_position,
                    ) {
                        combat.write(CombatMessage::Despawn(bullet_entity));
                    }
                }
            }
        }

        return;
    }

    {
        let CollisionCache {
            enemy_grid,
            hit_enemies,
            ..
        } = &mut *cache;

        for x in (enemy_x - R_PLAYER_ENEMY)..=(enemy_x + R_PLAYER_ENEMY) {
            for y in (enemy_y - R_PLAYER_ENEMY)..=(enemy_y + R_PLAYER_ENEMY) {
                let Some(entries) = enemy_grid.get_cell(x, y) else {
                    continue;
                };

                for &(enemy_entity, enemy_position, enemy_size) in entries {
                    if hit_enemies.contains(&enemy_entity)
                        || !player_collider.intersects(
                            player_position,
                            Collider { size: enemy_size },
                            enemy_position,
                        )
                    {
                        continue;
                    }

                    let defeated = health.damage(1);
                    hit_enemies.insert(enemy_entity);
                    combat.write(CombatMessage::PlayerDamaged {
                        player: player_entity,
                        defeated,
                        consumed: enemy_entity,
                    });
                    return;
                }
            }
        }
    }

    for x in (bullet_x - R_PLAYER_BULLET)..=(bullet_x + R_PLAYER_BULLET) {
        for y in (bullet_y - R_PLAYER_BULLET)..=(bullet_y + R_PLAYER_BULLET) {
            let Some(entries) = cache.enemy_bullet_grid.get_cell(x, y) else {
                continue;
            };

            for &(bullet_entity, bullet_position, bullet_size) in entries {
                if !player_collider.intersects(
                    player_position,
                    Collider { size: bullet_size },
                    bullet_position,
                ) {
                    continue;
                }

                let defeated = health.damage(1);
                combat.write(CombatMessage::PlayerDamaged {
                    player: player_entity,
                    defeated,
                    consumed: bullet_entity,
                });
                return;
            }
        }
    }
}

fn apply_combat_outcomes(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut next_state: ResMut<NextState<GameState>>,
    mut reader: MessageReader<CombatMessage>,
    mut shake: MessageWriter<ShakeEvent>,
    mut player_statuses: Query<&mut PlayerStatus>,
) {
    let mut despawned = EntityHashSet::default();
    let mut shake_magnitude: f32 = 0.0;
    let mut shake_duration: f32 = 0.0;

    for outcome in reader.read().copied() {
        match outcome {
            CombatMessage::Despawn(entity) => {
                despawn_once(&mut commands, &mut despawned, entity);
            }
            CombatMessage::EnemyHit(entity) => {
                commands.entity(entity).insert(EnemyHitFlash::new());
                shake_magnitude = shake_magnitude.max(2.0);
                shake_duration = shake_duration.max(0.03);
            }
            CombatMessage::EnemyDestroyed {
                entity,
                position,
                score: points,
                drop,
            } => {
                despawn_once(&mut commands, &mut despawned, entity);
                effects::spawn_explosion(&mut commands, position);
                score.0 += points;

                if let Some(kind) = drop {
                    powerup::spawn_pickup(&mut commands, position, kind);
                }

                shake_magnitude = shake_magnitude.max(4.0);
                shake_duration = shake_duration.max(0.06);
            }
            CombatMessage::PlayerDamaged {
                player,
                defeated,
                consumed,
            } => {
                despawn_once(&mut commands, &mut despawned, consumed);

                if defeated {
                    next_state.set(GameState::GameOver);
                    shake_magnitude = shake_magnitude.max(20.0);
                    shake_duration = shake_duration.max(0.3);
                } else {
                    if let Ok(mut status) = player_statuses.get_mut(player) {
                        status.grant_invincibility(INVINCIBILITY_SECONDS);
                    }
                    shake_magnitude = shake_magnitude.max(14.0);
                    shake_duration = shake_duration.max(0.15);
                }
            }
        }
    }

    if shake_magnitude > 0.0 && shake_duration > 0.0 {
        shake.write(ShakeEvent {
            magnitude: shake_magnitude,
            duration: shake_duration,
        });
    }
}

fn despawn_once(commands: &mut Commands, despawned: &mut EntityHashSet, entity: Entity) {
    if despawned.insert(entity) {
        commands.entity(entity).despawn();
    }
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
