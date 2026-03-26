use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use std::ops::Range;
use std::time::Duration;

use super::combat::{DespawnRequestedEvent, EnemyDestroyedEvent, PlayerDamagedEvent};
use super::{ResolveSet, SimulationSet};

const DEFAULT_HALF_WIDTH: f32 = 400.0;
const DEFAULT_HALF_HEIGHT: f32 = 300.0;
pub const OFFSCREEN_MARGIN: f32 = 50.0;
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

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Velocity(pub Vec2);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Collider {
    pub size: Vec2,
}

impl Collider {
    pub fn intersects(self, position: Vec3, other: Self, other_position: Vec3) -> bool {
        let a_min = position.truncate() - self.size / 2.0;
        let a_max = position.truncate() + self.size / 2.0;
        let b_min = other_position.truncate() - other.size / 2.0;
        let b_max = other_position.truncate() + other.size / 2.0;

        a_min.x < b_max.x && a_max.x > b_min.x && a_min.y < b_max.y && a_max.y > b_min.y
    }
}

#[derive(Component, Debug)]
pub struct Lifetime(pub Timer);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct OffscreenDespawn {
    pub margin: Vec2,
    check_x: bool,
    check_y: bool,
}

impl OffscreenDespawn {
    pub fn new(margin: Vec2) -> Self {
        Self {
            margin,
            check_x: true,
            check_y: true,
        }
    }

    pub fn horizontal(margin: f32) -> Self {
        Self {
            margin: Vec2::new(margin, 0.0),
            check_x: true,
            check_y: false,
        }
    }

    pub fn is_outside(self, bounds: &GameBounds, position: Vec3) -> bool {
        let x_limit = bounds.half_width + self.margin.x;
        let y_limit = bounds.half_height + self.margin.y;

        (self.check_x && position.x.abs() > x_limit) || (self.check_y && position.y.abs() > y_limit)
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

impl Health {
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }

    pub fn damage(&mut self, amount: u32) -> bool {
        self.current = self.current.saturating_sub(amount);
        self.current == 0
    }

    pub fn heal(&mut self, amount: u32) {
        self.current = (self.current + amount).min(self.max);
    }
}

#[derive(Component)]
pub struct InGameEntity;

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

pub struct GameCorePlugin;

impl Plugin for GameCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameBounds>()
            .init_resource::<Score>()
            .add_systems(Startup, sync_bounds)
            .add_systems(PreUpdate, sync_bounds_on_resize)
            .add_observer(handle_despawn_requests)
            .add_observer(cleanup_destroyed_enemy)
            .add_observer(cleanup_consumed_entity)
            .add_observer(track_destroyed_enemy_score)
            .add_systems(
                Update,
                apply_velocity.in_set(SimulationSet::Move),
            )
            .add_systems(
                Update,
                (tick_lifetimes, despawn_offscreen)
                    .chain()
                    .in_set(ResolveSet::Cleanup),
            );
    }
}

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

pub fn remaining_timer_secs(timer: &Timer) -> f32 {
    (timer.duration().as_secs_f32() - timer.elapsed_secs()).max(0.0)
}

fn sync_bounds(windows: Query<&Window, With<PrimaryWindow>>, mut bounds: ResMut<GameBounds>) {
    let Ok(window) = windows.single() else {
        return;
    };

    bounds.half_width = window.width() * 0.5;
    bounds.half_height = window.height() * 0.5;
}

fn sync_bounds_on_resize(
    reader: Option<MessageReader<WindowResized>>,
    mut bounds: ResMut<GameBounds>,
) {
    let Some(mut reader) = reader else {
        return;
    };

    for event in reader.read() {
        bounds.half_width = event.width * 0.5;
        bounds.half_height = event.height * 0.5;
    }
}

fn apply_velocity(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity)>) {
    let dt = capped_delta_seconds(&time);

    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;
    }
}

fn tick_lifetimes(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Lifetime)>,
) {
    for (entity, mut lifetime) in &mut query {
        lifetime.0.tick(time.delta());
        if lifetime.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn despawn_offscreen(
    mut commands: Commands,
    bounds: Res<GameBounds>,
    query: Query<(Entity, &Transform, &OffscreenDespawn)>,
) {
    for (entity, transform, policy) in &query {
        if policy.is_outside(&bounds, transform.translation) {
            commands.entity(entity).despawn();
        }
    }
}

fn handle_despawn_requests(event: On<DespawnRequestedEvent>, mut commands: Commands) {
    commands.entity(event.0).despawn();
}

fn cleanup_destroyed_enemy(event: On<EnemyDestroyedEvent>, mut commands: Commands) {
    commands.trigger(DespawnRequestedEvent(event.entity));
}

fn cleanup_consumed_entity(event: On<PlayerDamagedEvent>, mut commands: Commands) {
    commands.trigger(DespawnRequestedEvent(event.consumed));
}

fn track_destroyed_enemy_score(
    event: On<EnemyDestroyedEvent>,
    mut score: ResMut<Score>,
) {
    score.0 += event.score;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collider_detects_overlap() {
        let player = Collider {
            size: Vec2::new(10.0, 10.0),
        };
        let enemy = Collider {
            size: Vec2::new(8.0, 8.0),
        };

        assert!(player.intersects(Vec3::ZERO, enemy, Vec3::new(4.0, 0.0, 0.0)));
        assert!(!player.intersects(Vec3::ZERO, enemy, Vec3::new(20.0, 0.0, 0.0)));
    }

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

    #[test]
    fn offscreen_despawn_respects_horizontal_policy() {
        let bounds = GameBounds::default();
        let policy = OffscreenDespawn::horizontal(20.0);

        assert!(policy.is_outside(&bounds, Vec3::new(bounds.half_width + 25.0, 0.0, 0.0)));
        assert!(!policy.is_outside(&bounds, Vec3::new(0.0, bounds.half_height + 100.0, 0.0)));
    }
}
