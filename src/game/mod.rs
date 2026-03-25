#[cfg(target_arch = "wasm32")]
use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowPlugin, WindowResized, WindowResolution};

pub mod assets;
pub mod background;
pub mod combat;
pub mod effects;
pub mod enemy;
pub mod player;
pub mod shared;
pub mod state;
pub mod ui;

use shared::{GameBounds, MainCamera, Score};

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
        app.init_resource::<GameBounds>()
            .init_resource::<Score>()
            .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05)))
            .configure_sets(
                Update,
                (
                    GameplaySet::Input,
                    GameplaySet::Spawn,
                    GameplaySet::Movement,
                    GameplaySet::Collision,
                    GameplaySet::Cleanup,
                    GameplaySet::Ui,
                    GameplaySet::Fx,
                )
                    .chain(),
            )
            .add_systems(Startup, (setup_camera, update_bounds).chain())
            .add_systems(PreUpdate, update_bounds_from_resize)
            .add_plugins((
                state::StatePlugin,
                assets::AssetsPlugin,
                ui::UiPlugin,
                background::BackgroundPlugin,
                player::PlayerPlugin,
                enemy::EnemyPlugin,
                combat::CombatPlugin,
                effects::EffectsPlugin,
            ));
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameplaySet {
    Input,
    Spawn,
    Movement,
    Collision,
    Cleanup,
    Ui,
    Fx,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera::default()));
}

fn update_bounds(windows: Query<&Window, With<PrimaryWindow>>, mut bounds: ResMut<GameBounds>) {
    let Ok(window) = windows.single() else {
        return;
    };

    bounds.half_width = window.width() * 0.5;
    bounds.half_height = window.height() * 0.5;
}

fn update_bounds_from_resize(
    mut reader: MessageReader<WindowResized>,
    mut bounds: ResMut<GameBounds>,
) {
    for event in reader.read() {
        bounds.half_width = event.width * 0.5;
        bounds.half_height = event.height * 0.5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::assets::GameAssets;
    use crate::game::enemy::{Difficulty, SpawnState};
    use crate::game::state::{GameState, PlayState};

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
    }
}
