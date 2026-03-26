use bevy::prelude::*;

use super::super::core::{GameBounds, capped_delta_seconds};
use super::components::{Player, PlayerIntent};

const PLAYER_SPEED: f32 = 500.0;

pub fn capture_player_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut PlayerIntent, With<Player>>,
) {
    let Ok(mut intent) = query.single_mut() else {
        return;
    };

    let mut move_dir = Vec2::ZERO;
    if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
        move_dir.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
        move_dir.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
        move_dir.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
        move_dir.x += 1.0;
    }

    intent.move_dir = if move_dir.length_squared() > 0.0 {
        move_dir.normalize()
    } else {
        Vec2::ZERO
    };
    intent.firing = keyboard_input.pressed(KeyCode::Space);
}

pub fn apply_player_movement(
    time: Res<Time>,
    bounds: Res<GameBounds>,
    mut query: Query<(&PlayerIntent, &mut Transform), With<Player>>,
) {
    let Ok((intent, mut transform)) = query.single_mut() else {
        return;
    };

    let dt = capped_delta_seconds(&time);
    let (x_min, x_max) = bounds.player_x_range(20.0);
    let (y_min, y_max) = bounds.player_y_range(20.0);

    transform.translation.x =
        (transform.translation.x + intent.move_dir.x * PLAYER_SPEED * dt).clamp(x_min, x_max);
    transform.translation.y =
        (transform.translation.y + intent.move_dir.y * PLAYER_SPEED * dt).clamp(y_min, y_max);
}
