use bevy::prelude::*;
use std::time::Duration;

use super::super::core::{Health, InGameEntity, Score};
use super::super::player::{PLAYER_MAX_HP, Player, PlayerEffectSnapshot};
use super::TextBlockBundle;

const SCORE_FONT_SIZE: f32 = 30.0;
const POWERUP_FONT_SIZE: f32 = 24.0;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct HpText;

#[derive(Component)]
pub struct PowerUpText;

pub fn setup_hud(mut commands: Commands) {
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
        InGameEntity,
    ));

    commands.spawn((
        TextBlockBundle::new(
            hp_label(PLAYER_MAX_HP),
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
        InGameEntity,
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
        InGameEntity,
    ));
}

pub fn update_score_text(score: Res<Score>, mut text: Single<&mut Text, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }

    text.0 = score_label(score.0);
}

pub fn update_hp_text(
    player_health: Single<Ref<Health>, With<Player>>,
    mut text: Single<&mut Text, With<HpText>>,
) {
    if !player_health.is_changed() {
        return;
    }

    text.0 = hp_label(player_health.current);
}

pub fn update_powerup_ui(
    time: Res<Time>,
    player: Single<
        (
            Option<&'static super::super::player::TripleShot>,
            Option<&'static super::super::player::RapidFire>,
            Option<&'static super::super::player::PierceShot>,
            Option<&'static super::super::player::Invincible>,
        ),
        With<Player>,
    >,
    mut text: Single<&mut Text, With<PowerUpText>>,
    mut ui_timer: Local<Timer>,
) {
    if ui_timer.duration() == Duration::ZERO {
        *ui_timer = Timer::from_seconds(0.1, TimerMode::Repeating);
    }

    ui_timer.tick(time.delta());
    if !ui_timer.just_finished() {
        return;
    }

    let (triple_shot, rapid_fire, pierce_shot, invincible) = *player;
    let effects =
        PlayerEffectSnapshot::from_components(triple_shot, rapid_fire, pierce_shot, invincible);

    text.0 = powerup_lines(effects);
}

fn powerup_lines(effects: PlayerEffectSnapshot) -> String {
    let mut text = String::new();
    let mut first_line = true;

    append_effect_line(
        &mut text,
        &mut first_line,
        "TRIPLE",
        effects.triple_shot,
    );
    append_effect_line(
        &mut text,
        &mut first_line,
        "RAPID",
        effects.rapid_fire,
    );
    append_effect_line(
        &mut text,
        &mut first_line,
        "PIERCE",
        effects.pierce_shot,
    );
    append_effect_line(
        &mut text,
        &mut first_line,
        "INVINCIBLE",
        effects.invincible.map(|state| state.remaining_secs),
    );

    text
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

    buffer.push_str(&format!("{label}: {remaining:.1}s"));
    *first_line = false;
}

fn score_label(score: u32) -> String {
    format!("Score: {score}")
}

fn hp_label(hp: u32) -> String {
    format!("HP: {hp}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powerup_lines_skip_missing_effects() {
        let text = powerup_lines(PlayerEffectSnapshot::default());
        assert!(text.is_empty());
    }
}
