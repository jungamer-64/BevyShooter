use bevy::prelude::*;
use std::time::Duration;

use super::super::core::{Health, InGameEntity, Score};
use super::super::player::{Invincible, PLAYER_MAX_HP, PierceShot, Player, RapidFire, TripleShot};
use super::TextBlockBundle;

const SCORE_FONT_SIZE: f32 = 30.0;
const POWERUP_FONT_SIZE: f32 = 24.0;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct HpText;

#[derive(Component)]
pub struct PowerUpText;

type PlayerHudQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static TripleShot>,
        Option<&'static RapidFire>,
        Option<&'static PierceShot>,
        Option<&'static Invincible>,
    ),
    With<Player>,
>;

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

pub fn update_score_text(score: Res<Score>, mut query: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }

    let Ok(mut text) = query.single_mut() else {
        return;
    };

    text.0 = score_label(score.0);
}

pub fn update_hp_text(
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

    text.0 = hp_label(health.current);
}

pub fn update_powerup_ui(
    time: Res<Time>,
    player_query: PlayerHudQuery,
    mut text_query: Query<&mut Text, With<PowerUpText>>,
    mut ui_timer: Local<Timer>,
) {
    if ui_timer.duration() == Duration::ZERO {
        *ui_timer = Timer::from_seconds(0.1, TimerMode::Repeating);
    }

    ui_timer.tick(time.delta());
    if !ui_timer.just_finished() {
        return;
    }

    let Ok((triple_shot, rapid_fire, pierce_shot, invincible)) = player_query.single() else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    text.0 = powerup_lines(triple_shot, rapid_fire, pierce_shot, invincible);
}

fn powerup_lines(
    triple_shot: Option<&TripleShot>,
    rapid_fire: Option<&RapidFire>,
    pierce_shot: Option<&PierceShot>,
    invincible: Option<&Invincible>,
) -> String {
    let mut text = String::new();
    let mut first_line = true;

    append_effect_line(
        &mut text,
        &mut first_line,
        "TRIPLE",
        triple_shot.map(TripleShot::remaining_secs),
    );
    append_effect_line(
        &mut text,
        &mut first_line,
        "RAPID",
        rapid_fire.map(RapidFire::remaining_secs),
    );
    append_effect_line(
        &mut text,
        &mut first_line,
        "PIERCE",
        pierce_shot.map(PierceShot::remaining_secs),
    );
    append_effect_line(
        &mut text,
        &mut first_line,
        "INVINCIBLE",
        invincible.map(Invincible::remaining_secs),
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
        let text = powerup_lines(None, None, None, None);
        assert!(text.is_empty());
    }
}
