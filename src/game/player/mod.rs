mod components;
mod effects;
mod input;
mod shoot;

use bevy::prelude::*;

use super::assets::GameAssets;
use super::state::{GameState, PlayState};
use super::{GameplaySet, SimulationSet};

pub(crate) use components::{
    BULLET_SCALE, BULLET_SIZE, INVINCIBILITY_SECONDS, PLAYER_MAX_HP, PLAYER_SCALE, PLAYER_SIZE,
};
pub use components::{
    Bullet, Invincible, PierceShot, Player, PlayerBundle, PlayerWeapons, RapidFire,
    TimedEffectComponent, TripleShot,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_player)
            .add_systems(
                Update,
                input::capture_player_input
                    .in_set(GameplaySet::Input)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            )
            .add_systems(
                Update,
                (
                    effects::tick_player_cooldown,
                    effects::tick_temporary_effects,
                    input::apply_player_movement,
                    shoot::player_shoot,
                )
                    .chain()
                    .in_set(SimulationSet::Prepare)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

fn spawn_player(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(PlayerBundle::new(&game_assets));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn player_weapons_adjust_cooldown_for_rapid_fire() {
        let mut weapons = PlayerWeapons::new();

        weapons.reset_fire_cooldown(false);
        let normal_cooldown = weapons.fire_cooldown.duration();

        weapons.reset_fire_cooldown(true);
        let rapid_cooldown = weapons.fire_cooldown.duration();

        assert!(rapid_cooldown < normal_cooldown);
    }

    #[test]
    fn timed_effect_components_report_remaining_time() {
        let mut effect = TripleShot::new(1.0);
        effect.tick(Duration::from_secs_f32(0.4));

        assert!(effect.remaining_secs() > 0.5);
        assert!(!effect.is_finished());
    }
}
