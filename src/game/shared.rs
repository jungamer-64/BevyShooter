use bevy::prelude::*;
use std::ops::Range;
use std::time::Duration;

const DEFAULT_HALF_WIDTH: f32 = 400.0;
const DEFAULT_HALF_HEIGHT: f32 = 300.0;
const OFFSCREEN_MARGIN: f32 = 50.0;
const SPAWN_MARGIN: f32 = 50.0;
const DELTA_CAP_SECONDS: f32 = 1.0 / 30.0;

pub mod layer {
    pub const STARS: f32 = -10.0;
    pub const ENEMY: f32 = 0.0;
    pub const PLAYER: f32 = 1.0;
    pub const BULLET: f32 = 2.0;
    pub const POWERUP: f32 = 3.0;
    pub const FX: f32 = 5.0;
}

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Collider {
    pub size: Vec2,
}

#[derive(Component)]
pub struct Lifetime(pub Timer);

#[derive(Component)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

#[derive(Component)]
pub struct GameEntity;

#[derive(Component, Default)]
pub struct MainCamera {
    pub base: Vec3,
}

#[derive(Resource)]
pub struct GameBounds {
    pub half_width: f32,
    pub half_height: f32,
}

impl Default for GameBounds {
    fn default() -> Self {
        Self {
            half_width: DEFAULT_HALF_WIDTH,
            half_height: DEFAULT_HALF_HEIGHT,
        }
    }
}

impl GameBounds {
    pub fn player_x_range(&self, margin: f32) -> (f32, f32) {
        (-self.half_width + margin, self.half_width - margin)
    }

    pub fn player_y_range(&self, margin: f32) -> (f32, f32) {
        (-self.half_height + margin, self.half_height - margin)
    }

    pub fn spawn_x(&self) -> f32 {
        self.half_width + SPAWN_MARGIN
    }

    pub fn despawn_x(&self) -> f32 {
        -self.half_width - OFFSCREEN_MARGIN
    }

    pub fn spawn_y_range(&self) -> Range<f32> {
        let min = -self.half_height + SPAWN_MARGIN;
        let max = self.half_height - SPAWN_MARGIN;

        if min < max { min..max } else { 0.0..0.1 }
    }
}

#[derive(Resource, Default)]
pub struct Score(pub u32);

pub fn frand_range(range: Range<f32>) -> f32 {
    let t = fastrand::f32();
    range.start + t * (range.end - range.start)
}

pub fn capped_delta_seconds(time: &Time) -> f32 {
    time.delta_secs().min(DELTA_CAP_SECONDS)
}

pub fn ready_once_timer(seconds: f32) -> Timer {
    let mut timer = Timer::from_seconds(seconds, TimerMode::Once);
    timer.tick(Duration::from_secs_f32(seconds));
    timer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_ranges_follow_window_size() {
        let bounds = GameBounds {
            half_width: 512.0,
            half_height: 384.0,
        };

        assert_eq!(bounds.player_x_range(20.0), (-492.0, 492.0));
        assert_eq!(bounds.player_y_range(20.0), (-364.0, 364.0));
        assert_eq!(bounds.spawn_x(), 562.0);
        assert_eq!(bounds.despawn_x(), -562.0);
    }

    #[test]
    fn spawn_y_range_stays_valid_for_tiny_windows() {
        let bounds = GameBounds {
            half_width: 20.0,
            half_height: 10.0,
        };

        let range = bounds.spawn_y_range();
        assert_eq!(range.start, 0.0);
        assert_eq!(range.end, 0.1);
    }
}
