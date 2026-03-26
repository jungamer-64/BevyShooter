use bevy::prelude::*;

use super::state::{GameState, PlayState};

pub fn gameplay_active(
    game_state: Res<State<GameState>>,
    play_state: Res<State<PlayState>>,
) -> bool {
    *game_state.get() == GameState::InGame && *play_state.get() == PlayState::Playing
}
