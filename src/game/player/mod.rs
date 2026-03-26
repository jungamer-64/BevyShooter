mod components;
mod effects;
mod input;
mod shoot;

use bevy::prelude::*;

use super::assets::GameAssets;
use super::combat::PlayerDamagedEvent;
use super::state::GameState;
use super::{GameplaySet, SimulationSet};

pub(crate) use components::{
    BULLET_SCALE, BULLET_SIZE, INVINCIBILITY_SECONDS, PLAYER_MAX_HP, PLAYER_SCALE, PLAYER_SIZE,
};
pub use components::{
    Bullet, Invincible, PierceShot, Player, PlayerBundle, PlayerEffectSnapshot, PlayerWeapons,
    RapidFire, TimedEffectComponent, TripleShot,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_player)
            .add_observer(handle_player_damage)
            .add_systems(
                Update,
                input::capture_player_input.in_set(GameplaySet::Input),
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
                    .in_set(SimulationSet::Prepare),
            )
            .add_systems(
                Update,
                effects::update_invincibility_visuals.in_set(GameplaySet::Fx),
            );
    }
}

fn spawn_player(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(PlayerBundle::new(&game_assets));
}

fn handle_player_damage(event: On<PlayerDamagedEvent>, mut commands: Commands) {
    if !event.defeated {
        commands
            .entity(event.player)
            .insert(Invincible::new(INVINCIBILITY_SECONDS));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::combat::PlayerDamagedEvent;
    use bevy::app::App;
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

    #[test]
    fn surviving_player_damage_grants_invincibility() {
        let mut app = App::new();
        app.add_plugins(PlayerPlugin);

        let player = app.world_mut().spawn(Player).id();
        app.world_mut().trigger(PlayerDamagedEvent {
            player,
            defeated: false,
            consumed: player,
        });
        app.world_mut().flush();

        assert!(app.world().entity(player).contains::<Invincible>());
    }
}
