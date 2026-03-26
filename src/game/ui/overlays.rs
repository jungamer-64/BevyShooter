use bevy::prelude::*;

use super::super::core::Score;
use super::TextBlockBundle;

const OVERLAY_FONT_LARGE: f32 = 50.0;
const OVERLAY_FONT_MEDIUM: f32 = 40.0;

#[derive(Component)]
pub struct MenuUi;

#[derive(Component)]
pub struct GameOverUi;

#[derive(Component)]
pub struct PausedUi;

pub fn setup_menu(mut commands: Commands) {
    commands.spawn((
        TextBlockBundle::new(
            "BEVY SHOOTER\n\nPress SPACE to Start",
            OVERLAY_FONT_LARGE,
            Color::WHITE,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(20.0),
                left: Val::Percent(15.0),
                ..default()
            },
        ),
        MenuUi,
    ));
}

pub fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuUi>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn setup_gameover(mut commands: Commands, score: Res<Score>) {
    commands.spawn((
        TextBlockBundle::new(
            format!(
                "GAME OVER\n\nFinal Score: {}\n\nPress SPACE to Restart",
                score.0
            ),
            OVERLAY_FONT_MEDIUM,
            Color::srgb(1.0, 0.3, 0.3),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(30.0),
                left: Val::Percent(20.0),
                ..default()
            },
        ),
        GameOverUi,
    ));
}

pub fn cleanup_gameover(mut commands: Commands, query: Query<Entity, With<GameOverUi>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn setup_paused(mut commands: Commands) {
    commands.spawn((
        TextBlockBundle::new(
            "PAUSED\n\nPress ESC to Resume",
            OVERLAY_FONT_LARGE,
            Color::srgb(0.8, 0.8, 1.0),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(35.0),
                left: Val::Percent(30.0),
                ..default()
            },
        ),
        PausedUi,
    ));
}

pub fn cleanup_paused(mut commands: Commands, query: Query<Entity, With<PausedUi>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
