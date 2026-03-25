use bevy::prelude::*;
use core::fmt::Write as _;
use std::time::Duration;

use super::GameplaySet;
use super::player::{PierceShot, Player, RapidFire, TripleShot};
use super::shared::{GameEntity, Health, Score};
use super::state::{GameState, PlayState};

const SCORE_FONT_SIZE: f32 = 30.0;
const POWERUP_FONT_SIZE: f32 = 24.0;
const OVERLAY_FONT_LARGE: f32 = 50.0;
const OVERLAY_FONT_MEDIUM: f32 = 40.0;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct HpText;

#[derive(Component)]
pub struct MenuUi;

#[derive(Component)]
pub struct GameOverUi;

#[derive(Component)]
pub struct PausedUi;

#[derive(Component)]
pub struct PowerUpText;

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

type PowerUpTextQuery<'w, 's> =
    Query<'w, 's, &'static mut Text, (With<PowerUpText>, Without<ScoreText>, Without<HpText>)>;
type PlayerPowerUpQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static TripleShot>,
        Option<&'static RapidFire>,
        Option<&'static PierceShot>,
    ),
    With<Player>,
>;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_hud)
            .add_systems(
                Update,
                (update_score_text, update_hp_text, update_powerup_ui)
                    .in_set(GameplaySet::Ui)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

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

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        TextBlockBundle::new(
            score_label(0),
            SCORE_FONT_SIZE,
            Color::WHITE,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
        ),
        ScoreText,
        GameEntity,
    ));

    commands.spawn((
        TextBlockBundle::new(
            hp_label(super::player::PLAYER_MAX_HP),
            SCORE_FONT_SIZE,
            Color::srgb(0.2, 1.0, 0.2),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                ..default()
            },
        ),
        HpText,
        GameEntity,
    ));

    commands.spawn((
        TextBlockBundle::new(
            "",
            POWERUP_FONT_SIZE,
            Color::srgb(0.5, 0.8, 1.0),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(45.0),
                right: Val::Px(10.0),
                ..default()
            },
        ),
        PowerUpText,
        GameEntity,
    ));
}

fn update_score_text(score: Res<Score>, mut query: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }

    let Ok(mut text) = query.single_mut() else {
        return;
    };

    text.0.clear();
    let _ = write!(text.0, "{}", score_label(score.0));
}

fn update_hp_text(
    player_query: Query<Ref<Health>, With<Player>>,
    mut query: Query<&mut Text, With<HpText>>,
) {
    let Ok(health) = player_query.single() else {
        return;
    };

    if !health.is_changed() {
        return;
    }

    let Ok(mut text) = query.single_mut() else {
        return;
    };

    text.0.clear();
    let _ = write!(text.0, "{}", hp_label(health.current));
}

fn update_powerup_ui(
    time: Res<Time>,
    player_query: PlayerPowerUpQuery,
    mut text_query: PowerUpTextQuery,
    mut ui_timer: Local<Timer>,
) {
    if ui_timer.duration() == Duration::ZERO {
        *ui_timer = Timer::from_seconds(0.1, TimerMode::Repeating);
    }

    ui_timer.tick(time.delta());
    if !ui_timer.just_finished() {
        return;
    }

    let Ok((triple, rapid, pierce)) = player_query.single() else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    text.0.clear();
    let mut first_line = true;

    append_effect_line(
        &mut text.0,
        &mut first_line,
        "TRIPLE",
        triple.map(TripleShot::remaining_secs),
    );
    append_effect_line(
        &mut text.0,
        &mut first_line,
        "RAPID",
        rapid.map(RapidFire::remaining_secs),
    );
    append_effect_line(
        &mut text.0,
        &mut first_line,
        "PIERCE",
        pierce.map(PierceShot::remaining_secs),
    );
}

fn append_effect_line(
    buffer: &mut String,
    first_line: &mut bool,
    label: &str,
    remaining: Option<f32>,
) {
    let Some(remaining) = remaining else {
        return;
    };

    if remaining <= 0.0 {
        return;
    }

    if !*first_line {
        buffer.push('\n');
    }

    let _ = write!(buffer, "{}: {:.1}s", label, remaining);
    *first_line = false;
}

fn score_label(score: u32) -> String {
    format!("Score: {score}")
}

fn hp_label(hp: u32) -> String {
    format!("HP: {hp}")
}
