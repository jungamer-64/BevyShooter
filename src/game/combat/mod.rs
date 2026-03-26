mod detect;
mod events;
mod resolve;
mod spatial;

use bevy::prelude::*;

use super::state::{GameState, PlayState};
use super::{GameplaySet, ResolveSet};

pub use detect::{HitList, Pierce};
pub use events::{
    BulletEnemyContact, DespawnRequest, EnemyDestroyed, EnemyHit, PlayerBulletContact,
    PlayerDamaged, PlayerEnemyContact,
};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<spatial::CollisionCache>()
            .add_message::<BulletEnemyContact>()
            .add_message::<PlayerEnemyContact>()
            .add_message::<PlayerBulletContact>()
            .add_message::<DespawnRequest>()
            .add_message::<EnemyHit>()
            .add_message::<EnemyDestroyed>()
            .add_message::<PlayerDamaged>()
            .add_systems(
                Update,
                (
                    spatial::prepare_collision_cache,
                    detect::detect_bullet_enemy_collisions,
                    detect::detect_player_collisions,
                )
                    .chain()
                    .in_set(GameplaySet::Detect)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            )
            .add_systems(
                Update,
                (
                    resolve::resolve_bullet_enemy_contacts,
                    resolve::resolve_player_contacts,
                    resolve::apply_combat_outcomes,
                )
                    .chain()
                    .in_set(ResolveSet::Apply)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::core::{Collider, GameBounds, Health, Score};
    use crate::game::effects::ShakeEvent;
    use crate::game::enemy::{Enemy, EnemyBullet, EnemyType};
    use crate::game::player::{Bullet, Invincible, Player};
    use crate::game::state::{GameState, PlayState};
    use bevy::app::App;
    use bevy::state::app::StatesPlugin;

    fn combat_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.insert_state(GameState::InGame);
        app.insert_state(PlayState::Playing);
        app.insert_resource(GameBounds::default());
        app.insert_resource(Score(0));
        app.add_message::<ShakeEvent>();
        app.add_plugins(CombatPlugin);
        app
    }

    #[test]
    fn hit_list_prevents_duplicate_hits() {
        let entity = Entity::from_raw_u32(7).expect("nonzero raw entity");
        let mut hit_list = HitList::default();

        assert!(!hit_list.contains(entity));
        hit_list.push(entity);
        assert!(hit_list.contains(entity));
    }

    #[test]
    fn multiple_bullet_contacts_only_destroy_enemy_once() {
        let mut app = combat_test_app();

        app.world_mut().spawn((
            Enemy,
            EnemyType::Normal,
            Health::new(1),
            Transform::default(),
            Collider {
                size: Vec2::splat(20.0),
            },
        ));

        app.world_mut().spawn((
            Bullet,
            Transform::default(),
            Collider {
                size: Vec2::splat(10.0),
            },
        ));

        app.world_mut().spawn((
            Bullet,
            Transform::default(),
            Collider {
                size: Vec2::splat(10.0),
            },
        ));

        app.update();
        app.update();

        assert_eq!(app.world().resource::<Score>().0, 10);
        let enemy_count = {
            let mut query = app.world_mut().query::<&Enemy>();
            query.iter(app.world()).count()
        };
        assert_eq!(enemy_count, 0);
    }

    #[test]
    fn invincible_player_consumes_enemy_and_bullet_without_damage() {
        let mut app = combat_test_app();

        app.world_mut().spawn((
            Player,
            Transform::default(),
            Collider {
                size: Vec2::splat(20.0),
            },
            Health::new(3),
            Invincible::new(1.0),
        ));

        app.world_mut().spawn((
            Enemy,
            EnemyType::Normal,
            Health::new(1),
            Transform::default(),
            Collider {
                size: Vec2::splat(20.0),
            },
        ));

        app.world_mut().spawn((
            EnemyBullet,
            Transform::default(),
            Collider {
                size: Vec2::splat(10.0),
            },
        ));

        app.update();
        app.update();

        let player_health = {
            let mut query = app.world_mut().query_filtered::<&Health, With<Player>>();
            query.single(app.world()).expect("player health").current
        };
        assert_eq!(player_health, 3);
        assert_eq!(app.world().resource::<Score>().0, 5);
        let enemy_count = {
            let mut query = app.world_mut().query::<&Enemy>();
            query.iter(app.world()).count()
        };
        let bullet_count = {
            let mut query = app.world_mut().query::<&EnemyBullet>();
            query.iter(app.world()).count()
        };
        assert_eq!(enemy_count, 0);
        assert_eq!(bullet_count, 0);
    }
}
