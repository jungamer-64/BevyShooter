use bevy::prelude::*;

use super::super::core::Collider;
use super::super::enemy::{ENEMY_BULLET_SIZE, ENEMY_SCALE, ENEMY_SIZE};
use super::super::player::{
    BULLET_SCALE, BULLET_SIZE, Bullet, Invincible, PLAYER_SCALE, PLAYER_SIZE, Player,
};
use super::events::{BulletEnemyContact, PlayerBulletContact, PlayerEnemyContact};
use super::spatial::{CollisionCache, SpatialGrid};

const R_BULLET_ENEMY: i32 = 2;
const R_PLAYER_ENEMY: i32 = 2;
const R_PLAYER_BULLET: i32 = 2;

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

pub fn detect_bullet_enemy_collisions(
    cache: Res<CollisionCache>,
    mut contacts: MessageWriter<BulletEnemyContact>,
    bullets: Query<
        (
            Entity,
            &Transform,
            &Collider,
            Option<&Pierce>,
            Option<&HitList>,
        ),
        With<Bullet>,
    >,
) {
    debug_assert_collision_radii();

    'bullet_loop: for (bullet_entity, bullet_transform, bullet_collider, _pierce, hit_list) in
        &bullets
    {
        let bullet_position = bullet_transform.translation;
        let (cx, cy) = cache.enemy_grid().cell_coords(bullet_position);

        for x in (cx - R_BULLET_ENEMY)..=(cx + R_BULLET_ENEMY) {
            for y in (cy - R_BULLET_ENEMY)..=(cy + R_BULLET_ENEMY) {
                let Some(entries) = cache.enemy_grid().get_cell(x, y) else {
                    continue;
                };

                for &(enemy_entity, enemy_position, enemy_size) in entries {
                    if let Some(existing_hits) = hit_list
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

                    contacts.write(BulletEnemyContact {
                        bullet: bullet_entity,
                        enemy: enemy_entity,
                    });

                    continue 'bullet_loop;
                }
            }
        }
    }
}

pub fn detect_player_collisions(
    cache: Res<CollisionCache>,
    mut enemy_contacts: MessageWriter<PlayerEnemyContact>,
    mut bullet_contacts: MessageWriter<PlayerBulletContact>,
    player_query: Query<(Entity, &Transform, &Collider, Option<&Invincible>), With<Player>>,
) {
    let Ok((player_entity, player_transform, player_collider, invincible)) = player_query.single()
    else {
        return;
    };

    let player_position = player_transform.translation;
    let (enemy_x, enemy_y) = cache.enemy_grid().cell_coords(player_position);
    let (bullet_x, bullet_y) = cache.enemy_bullet_grid().cell_coords(player_position);
    let invincible = invincible.is_some();

    for x in (enemy_x - R_PLAYER_ENEMY)..=(enemy_x + R_PLAYER_ENEMY) {
        for y in (enemy_y - R_PLAYER_ENEMY)..=(enemy_y + R_PLAYER_ENEMY) {
            let Some(entries) = cache.enemy_grid().get_cell(x, y) else {
                continue;
            };

            for &(enemy_entity, enemy_position, enemy_size) in entries {
                if !player_collider.intersects(
                    player_position,
                    Collider { size: enemy_size },
                    enemy_position,
                ) {
                    continue;
                }

                enemy_contacts.write(PlayerEnemyContact {
                    player: player_entity,
                    enemy: enemy_entity,
                });

                if !invincible {
                    return;
                }
            }
        }
    }

    for x in (bullet_x - R_PLAYER_BULLET)..=(bullet_x + R_PLAYER_BULLET) {
        for y in (bullet_y - R_PLAYER_BULLET)..=(bullet_y + R_PLAYER_BULLET) {
            let Some(entries) = cache.enemy_bullet_grid().get_cell(x, y) else {
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

                bullet_contacts.write(PlayerBulletContact {
                    player: player_entity,
                    bullet: bullet_entity,
                });

                if !invincible {
                    return;
                }
            }
        }
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
