use bevy::prelude::*;

use super::effects;
use super::shared::{GameEntity, Score};
use super::ui;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    Menu,
    InGame,
    GameOver,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum PlayState {
    #[default]
    Playing,
    Paused,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_state::<PlayState>()
            .add_systems(OnEnter(GameState::Menu), ui::setup_menu)
            .add_systems(Update, menu_input.run_if(in_state(GameState::Menu)))
            .add_systems(OnExit(GameState::Menu), ui::cleanup_menu)
            .add_systems(OnEnter(GameState::InGame), enter_game)
            .add_systems(
                OnExit(GameState::InGame),
                (cleanup_game_entities, effects::reset_camera_shake),
            )
            .add_systems(Update, pause_input.run_if(in_state(GameState::InGame)))
            .add_systems(
                OnEnter(PlayState::Paused),
                (ui::setup_paused, effects::reset_camera_shake),
            )
            .add_systems(OnExit(PlayState::Paused), ui::cleanup_paused)
            .add_systems(OnEnter(GameState::GameOver), ui::setup_gameover)
            .add_systems(Update, gameover_input.run_if(in_state(GameState::GameOver)))
            .add_systems(OnExit(GameState::GameOver), ui::cleanup_gameover);
    }
}

fn enter_game(mut score: ResMut<Score>, mut next_play_state: ResMut<NextState<PlayState>>) {
    score.0 = 0;
    next_play_state.set(PlayState::Playing);
}

fn cleanup_game_entities(mut commands: Commands, query: Query<Entity, With<GameEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn pause_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    play_state: Res<State<PlayState>>,
    mut next_play_state: ResMut<NextState<PlayState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match play_state.get() {
            PlayState::Playing => next_play_state.set(PlayState::Paused),
            PlayState::Paused => next_play_state.set(PlayState::Playing),
        }
    }
}

fn menu_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        next_state.set(GameState::InGame);
    }
}

fn gameover_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        next_state.set(GameState::InGame);
    }
}
