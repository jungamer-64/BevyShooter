#[cfg(target_arch = "wasm32")]
use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy::window::{WindowPlugin, WindowResolution};

pub mod assets;
pub mod background;
pub mod combat;
pub mod conditions;
pub mod core;
pub mod effects;
pub mod enemy;
pub mod player;
pub mod powerup;
pub mod state;
pub mod ui;

use core::MainCamera;

pub fn run() {
    #[allow(unused_mut)]
    let mut window = Window {
        title: "Bevy Shooter".into(),
        resolution: WindowResolution::new(800, 600),
        ..default()
    };

    #[cfg(target_arch = "wasm32")]
    {
        window.canvas = Some("#bevy".to_string());
        window.fit_canvas_to_parent = true;
        window.prevent_default_event_handling = true;
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(window),
                    ..default()
                })
                .set(AssetPlugin {
                    #[cfg(target_arch = "wasm32")]
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins(GamePlugin)
        .run();
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05)))
            .configure_sets(
                Update,
                (
                    GameplaySet::Input,
                    GameplaySet::Simulate,
                    GameplaySet::Detect,
                    GameplaySet::Resolve,
                    GameplaySet::Ui,
                    GameplaySet::Fx,
                )
                    .chain()
                    .run_if(conditions::gameplay_active),
            )
            .configure_sets(
                Update,
                (
                    SimulationSet::Prepare,
                    SimulationSet::Move,
                    SimulationSet::PostMove,
                )
                    .chain()
                    .in_set(GameplaySet::Simulate),
            )
            .configure_sets(
                Update,
                (ResolveSet::Apply, ResolveSet::Cleanup)
                    .chain()
                    .in_set(GameplaySet::Resolve),
            )
            .add_systems(Startup, setup_camera)
            .add_plugins((
                core::GameCorePlugin,
                state::StatePlugin,
                assets::AssetsPlugin,
                ui::UiPlugin,
                background::BackgroundPlugin,
                player::PlayerPlugin,
                powerup::PowerUpPlugin,
                enemy::EnemyPlugin,
                combat::CombatPlugin,
                effects::EffectsPlugin,
            ));
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameplaySet {
    Input,
    Simulate,
    Detect,
    Resolve,
    Ui,
    Fx,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimulationSet {
    Prepare,
    Move,
    PostMove,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolveSet {
    Apply,
    Cleanup,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera::default()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::assets::GameAssets;
    use crate::game::combat::{
        BulletEnemyContact, PlayerBulletContact, PlayerEnemyContact,
    };
    use crate::game::core::{GameBounds, Score};
    use crate::game::enemy::{Difficulty, SpawnState};
    use crate::game::state::{GameState, PlayState};
    use bevy::ecs::message::Messages;

    #[test]
    fn game_plugin_registers_core_resources_and_states() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(GamePlugin);

        assert!(app.world().contains_resource::<State<GameState>>());
        assert!(app.world().contains_resource::<State<PlayState>>());
        assert!(app.world().contains_resource::<GameBounds>());
        assert!(app.world().contains_resource::<Score>());
        assert!(app.world().contains_resource::<GameAssets>());
        assert!(app.world().contains_resource::<SpawnState>());
        assert!(app.world().contains_resource::<Difficulty>());
        assert!(
            app.world()
                .contains_resource::<Messages<BulletEnemyContact>>()
        );
        assert!(
            app.world()
                .contains_resource::<Messages<PlayerEnemyContact>>()
        );
        assert!(
            app.world()
                .contains_resource::<Messages<PlayerBulletContact>>()
        );
    }
}
