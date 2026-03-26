use bevy::prelude::*;

use super::super::core::{GameBounds, Velocity};
use super::super::player::Player;
use super::components::{ChasePlayer, Enemy, ZigzagMotion};

const ZIGZAG_FREQUENCY: f32 = 4.0;
const ZIGZAG_AMPLITUDE: f32 = 150.0;
const CHASER_MAX_VERTICAL_SPEED: f32 = 120.0;

pub fn update_enemy_velocity(
    time: Res<Time>,
    mut query: Query<
        (
            &Transform,
            &mut Velocity,
            Option<&ZigzagMotion>,
            Option<&ChasePlayer>,
        ),
        With<Enemy>,
    >,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    let player_y = player_query
        .single()
        .map(|transform| transform.translation.y)
        .unwrap_or(0.0);

    for (transform, mut velocity, zigzag, chaser) in &mut query {
        velocity.0.y = if let Some(zigzag) = zigzag {
            (time.elapsed_secs() * ZIGZAG_FREQUENCY + transform.translation.x * 0.01 + zigzag.phase)
                .sin()
                * ZIGZAG_AMPLITUDE
        } else if chaser.is_some() {
            (player_y - transform.translation.y)
                .clamp(-CHASER_MAX_VERTICAL_SPEED, CHASER_MAX_VERTICAL_SPEED)
        } else {
            0.0
        };
    }
}

pub fn clamp_enemy_position(
    bounds: Res<GameBounds>,
    mut query: Query<&mut Transform, With<Enemy>>,
) {
    let (y_min, y_max) = bounds.player_y_range(20.0);

    for mut transform in &mut query {
        transform.translation.y = transform.translation.y.clamp(y_min, y_max);
    }
}
