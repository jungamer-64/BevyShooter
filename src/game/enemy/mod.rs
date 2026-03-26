mod behavior;
mod components;
mod fire;
mod spawn;
mod visuals;

use bevy::prelude::*;

use super::state::GameState;
use super::{GameplaySet, SimulationSet};

pub use components::{Difficulty, Enemy, EnemyBullet, EnemyType, SpawnState};
pub(crate) use components::{ENEMY_BULLET_SIZE, ENEMY_SCALE, ENEMY_SIZE};
pub use spawn::{enemy_fire_interval_for_level, spawn_interval_for_level};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnState>()
            .init_resource::<Difficulty>()
            .add_systems(OnEnter(GameState::InGame), spawn::reset_enemy_progress)
            .add_observer(visuals::on_enemy_hit)
            .add_systems(
                Update,
                (
                    spawn::update_difficulty,
                    spawn::sync_spawn_interval,
                    spawn::enemy_spawner,
                    behavior::update_enemy_velocity,
                    fire::enemy_fire_system,
                )
                    .chain()
                    .in_set(SimulationSet::Prepare),
            )
            .add_systems(
                Update,
                behavior::clamp_enemy_position.in_set(SimulationSet::PostMove),
            )
            .add_systems(
                Update,
                visuals::update_enemy_visuals.in_set(GameplaySet::Fx),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn enemy_type_descriptors_match_expected_balance() {
        assert_eq!(EnemyType::Normal.initial_hp(), 1);
        assert_eq!(EnemyType::Zigzag.initial_hp(), 2);
        assert_eq!(EnemyType::Chaser.initial_hp(), 3);
        assert_eq!(EnemyType::Normal.score(), 10);
        assert_eq!(EnemyType::Zigzag.score(), 20);
        assert_eq!(EnemyType::Chaser.score(), 30);
    }

    #[test]
    fn difficulty_helpers_clamp_to_limits() {
        assert_eq!(spawn_interval_for_level(0), 1.0);
        assert_eq!(spawn_interval_for_level(10), 0.3);
        assert_eq!(enemy_fire_interval_for_level(0), 2.0);
        assert_eq!(enemy_fire_interval_for_level(10), 0.5);
    }

    #[test]
    fn spawn_timer_rescale_preserves_progress_fraction() {
        let mut spawn = SpawnState::default();
        spawn.timer.tick(Duration::from_secs_f32(0.4));

        spawn::retime_spawn_state(&mut spawn, 0.5);

        assert_eq!(spawn.current_interval, 0.5);
        assert!((spawn.timer.elapsed_secs() - 0.2).abs() < 1e-4);
    }
}
