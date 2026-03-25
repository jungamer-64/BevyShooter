use bevy::prelude::*;

use super::shared::{GameBounds, capped_delta_seconds, frand_range, layer};
use super::state::{GameState, PlayState};

const STAR_COUNT: usize = 50;
const STAR_SPEED_MIN: f32 = 50.0;
const STAR_SPEED_MAX: f32 = 200.0;

#[derive(Component)]
struct Star {
    speed: f32,
}

#[derive(Bundle)]
struct StarBundle {
    sprite: Sprite,
    transform: Transform,
    star: Star,
}

impl StarBundle {
    fn random(bounds: &GameBounds) -> Self {
        let x = frand_range(-bounds.half_width..bounds.half_width);
        let y = frand_range(-bounds.half_height..bounds.half_height);
        let speed = frand_range(STAR_SPEED_MIN..STAR_SPEED_MAX);
        let size = frand_range(1.0..4.0);
        let brightness = frand_range(0.3..1.0);

        Self {
            sprite: Sprite::from_color(
                Color::srgba(brightness, brightness, brightness, 1.0),
                Vec2::splat(size),
            ),
            transform: Transform::from_xyz(x, y, layer::STARS),
            star: Star { speed },
        }
    }
}

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_stars).add_systems(
            Update,
            update_stars.run_if(
                in_state(PlayState::Playing)
                    .or(in_state(GameState::Menu))
                    .or(in_state(GameState::GameOver)),
            ),
        );
    }
}

fn spawn_stars(mut commands: Commands, bounds: Res<GameBounds>) {
    for _ in 0..STAR_COUNT {
        commands.spawn(StarBundle::random(&bounds));
    }
}

fn update_stars(
    time: Res<Time>,
    bounds: Res<GameBounds>,
    mut query: Query<(&mut Transform, &Star)>,
) {
    let dt = capped_delta_seconds(&time);
    for (mut transform, star) in &mut query {
        transform.translation.x -= star.speed * dt;

        if transform.translation.x < bounds.despawn_x() {
            transform.translation.x = bounds.spawn_x();
            transform.translation.y = frand_range(-bounds.half_height..bounds.half_height);
        }
    }
}
