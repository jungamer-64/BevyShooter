use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;

use super::super::core::{Health, Score};
use super::super::effects::{self, ShakeEvent};
use super::super::enemy::{Enemy, EnemyHitFlash, EnemyType};
use super::super::player::{INVINCIBILITY_SECONDS, Invincible, Player};
use super::super::powerup;
use super::super::state::GameState;
use super::events::{
    BulletEnemyContact, DespawnRequest, EnemyDestroyed, EnemyHit, PlayerBulletContact,
    PlayerDamaged, PlayerEnemyContact,
};
use super::{HitList, Pierce};

pub fn resolve_bullet_enemy_contacts(
    mut contacts: MessageReader<BulletEnemyContact>,
    mut despawns: MessageWriter<DespawnRequest>,
    mut enemy_hits: MessageWriter<EnemyHit>,
    mut enemy_destroyed: MessageWriter<EnemyDestroyed>,
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
            enemy_destroyed.write(EnemyDestroyed {
                entity: contact.enemy,
                position: transform.translation,
                score: enemy_type.score(),
                drop: powerup::roll_drop(),
            });
        } else {
            enemy_hits.write(EnemyHit(contact.enemy));
        }

        if let Some(mut pierce) = pierce {
            if pierce.0 > 0 {
                pierce.0 -= 1;
            } else if consumed_bullets.insert(contact.bullet) {
                despawns.write(DespawnRequest(contact.bullet));
            }
        } else if consumed_bullets.insert(contact.bullet) {
            despawns.write(DespawnRequest(contact.bullet));
        }
    }
}

pub fn resolve_player_contacts(
    mut enemy_contacts: MessageReader<PlayerEnemyContact>,
    mut bullet_contacts: MessageReader<PlayerBulletContact>,
    mut despawns: MessageWriter<DespawnRequest>,
    mut player_damaged: MessageWriter<PlayerDamaged>,
    mut enemy_destroyed: MessageWriter<EnemyDestroyed>,
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
            enemy_destroyed.write(EnemyDestroyed {
                entity: contact.enemy,
                position: transform.translation,
                score: 5,
                drop: None,
            });
            continue;
        }

        let defeated = health.damage(1);
        handled_players.insert(contact.player);
        player_damaged.write(PlayerDamaged {
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
                despawns.write(DespawnRequest(contact.bullet));
            }
            continue;
        }

        let defeated = health.damage(1);
        handled_players.insert(contact.player);
        player_damaged.write(PlayerDamaged {
            player: contact.player,
            defeated,
            consumed: contact.bullet,
        });
    }
}

pub fn apply_combat_outcomes(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut next_state: ResMut<NextState<GameState>>,
    mut despawns: MessageReader<DespawnRequest>,
    mut enemy_hits: MessageReader<EnemyHit>,
    mut enemy_destroyed: MessageReader<EnemyDestroyed>,
    mut player_damaged: MessageReader<PlayerDamaged>,
    mut shake: MessageWriter<ShakeEvent>,
) {
    let mut despawned = EntityHashSet::default();
    let mut shake_magnitude: f32 = 0.0;
    let mut shake_duration: f32 = 0.0;

    for DespawnRequest(entity) in despawns.read().copied() {
        despawn_once(&mut commands, &mut despawned, entity);
    }

    for EnemyHit(entity) in enemy_hits.read().copied() {
        commands.entity(entity).insert(EnemyHitFlash::new());
        shake_magnitude = shake_magnitude.max(2.0);
        shake_duration = shake_duration.max(0.03);
    }

    for outcome in enemy_destroyed.read().copied() {
        despawn_once(&mut commands, &mut despawned, outcome.entity);
        effects::spawn_explosion(&mut commands, outcome.position);
        score.0 += outcome.score;

        if let Some(kind) = outcome.drop {
            powerup::spawn_pickup(&mut commands, outcome.position, kind);
        }

        shake_magnitude = shake_magnitude.max(4.0);
        shake_duration = shake_duration.max(0.06);
    }

    for outcome in player_damaged.read().copied() {
        despawn_once(&mut commands, &mut despawned, outcome.consumed);

        if outcome.defeated {
            next_state.set(GameState::GameOver);
            shake_magnitude = shake_magnitude.max(20.0);
            shake_duration = shake_duration.max(0.3);
        } else {
            commands
                .entity(outcome.player)
                .insert(Invincible::new(INVINCIBILITY_SECONDS));
            shake_magnitude = shake_magnitude.max(14.0);
            shake_duration = shake_duration.max(0.15);
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
