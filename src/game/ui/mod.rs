mod hud;
mod overlays;

use bevy::prelude::*;

use super::state::{GameState, PlayState};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), hud::setup_hud)
            .add_systems(OnEnter(GameState::Menu), overlays::setup_menu)
            .add_systems(OnExit(GameState::Menu), overlays::cleanup_menu)
            .add_systems(OnEnter(GameState::GameOver), overlays::setup_gameover)
            .add_systems(OnExit(GameState::GameOver), overlays::cleanup_gameover)
            .add_systems(OnEnter(PlayState::Paused), overlays::setup_paused)
            .add_systems(OnExit(PlayState::Paused), overlays::cleanup_paused)
            .add_systems(
                Update,
                (
                    hud::update_score_text,
                    hud::update_hp_text,
                    hud::update_powerup_ui,
                )
                    .in_set(super::GameplaySet::Ui)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

#[derive(Bundle)]
struct TextBlockBundle {
    text: Text,
    font: TextFont,
    color: TextColor,
    node: Node,
}

impl TextBlockBundle {
    fn new(text: impl Into<String>, font_size: f32, color: Color, node: Node) -> Self {
        Self {
            text: Text::new(text),
            font: TextFont {
                font_size,
                ..default()
            },
            color: TextColor(color),
            node,
        }
    }
}
