use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;

use super::super::core::Health;
use super::super::enemy::{Enemy, EnemyType};
use super::super::player::{Invincible, Player};
use super::events::{
    BulletEnemyContact, DespawnRequestedEvent, EnemyDestroyedEvent, EnemyHitEvent,
    PlayerBulletContact, PlayerDamagedEvent, PlayerEnemyContact,
};
use super::{HitList, Pierce};

pub fn resolve_bullet_enemy_contacts(
    mut commands: Commands,
    mut contacts: MessageReader<BulletEnemyContact>,
    mut bullets: Query<(Option<&mut Pierce>, Option<&mut HitList>)>,
    mut enemies: Query<(&mut Health, &EnemyType, &Transform), With<Enemy>>,
) {
    let mut destroyed_enemies = EntityHashSet::default();
    let mut consumed_bullets = EntityHashSet::default();

    for contact in contacts.read().copied() {
        if destroyed_enemies.contains(&contact.enemy) || consumed_bullets.contains(&contact.bullet)
        {
            continue;
        }

        let Ok((pierce, hit_list)) = bullets.get_mut(contact.bullet) else {
            continue;
        };
        let Ok((mut health, enemy_type, transform)) = enemies.get_mut(contact.enemy) else {
            continue;
        };

        if health.current == 0 {
            continue;
        }

        if hit_list
            .as_deref()
            .is_some_and(|existing_hits| existing_hits.contains(contact.enemy))
        {
            continue;
        }

        let destroyed = health.damage(1);

        if let Some(mut hit_list) = hit_list {
            hit_list.push(contact.enemy);
        }

        if destroyed {
            destroyed_enemies.insert(contact.enemy);
            commands.trigger(EnemyDestroyedEvent {
                entity: contact.enemy,
                position: transform.translation,
                score: enemy_type.score(),
            });
        } else {
            commands.trigger(EnemyHitEvent(contact.enemy));
        }

        if let Some(mut pierce) = pierce {
            if pierce.0 > 0 {
                pierce.0 -= 1;
            } else if consumed_bullets.insert(contact.bullet) {
                commands.trigger(DespawnRequestedEvent(contact.bullet));
            }
        } else if consumed_bullets.insert(contact.bullet) {
            commands.trigger(DespawnRequestedEvent(contact.bullet));
        }
    }
}

pub fn resolve_player_contacts(
    mut commands: Commands,
    mut enemy_contacts: MessageReader<PlayerEnemyContact>,
    mut bullet_contacts: MessageReader<PlayerBulletContact>,
    mut players: Query<(&mut Health, Option<&Invincible>), (With<Player>, Without<Enemy>)>,
    enemies: Query<(&Transform, &Health), (With<Enemy>, Without<Player>)>,
) {
    let mut handled_players = EntityHashSet::default();
    let mut destroyed_enemies = EntityHashSet::default();
    let mut despawned_bullets = EntityHashSet::default();

    for contact in enemy_contacts.read().copied() {
        if handled_players.contains(&contact.player) || destroyed_enemies.contains(&contact.enemy) {
            continue;
        }

        let Ok((mut health, invincible)) = players.get_mut(contact.player) else {
            continue;
        };
        let Ok((transform, enemy_health)) = enemies.get(contact.enemy) else {
            continue;
        };

        if enemy_health.current == 0 {
            continue;
        }

        if invincible.is_some() {
            destroyed_enemies.insert(contact.enemy);
            commands.trigger(EnemyDestroyedEvent {
                entity: contact.enemy,
                position: transform.translation,
                score: 5,
            });
            continue;
        }

        let defeated = health.damage(1);
        handled_players.insert(contact.player);
        commands.trigger(PlayerDamagedEvent {
            player: contact.player,
            defeated,
            consumed: contact.enemy,
        });
    }

    for contact in bullet_contacts.read().copied() {
        if handled_players.contains(&contact.player) || despawned_bullets.contains(&contact.bullet)
        {
            continue;
        }

        let Ok((mut health, invincible)) = players.get_mut(contact.player) else {
            continue;
        };

        if invincible.is_some() {
            if despawned_bullets.insert(contact.bullet) {
                commands.trigger(DespawnRequestedEvent(contact.bullet));
            }
            continue;
        }

        let defeated = health.damage(1);
        handled_players.insert(contact.player);
        commands.trigger(PlayerDamagedEvent {
            player: contact.player,
            defeated,
            consumed: contact.bullet,
        });
    }
}
