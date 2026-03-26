use bevy::prelude::*;

use super::super::assets::GameAssets;
use super::super::core::{
    Collider, GameBounds, Health, InGameEntity, Lifetime, OffscreenDespawn, Velocity, layer,
};

const ENEMY_SPEED: f32 = 300.0;
const BULLET_LIFETIME_SECONDS: f32 = 3.0;

pub(crate) const ENEMY_BULLET_SPEED: f32 = 400.0;
pub const ENEMY_SIZE: Vec2 = Vec2::new(30.0, 30.0);
pub const ENEMY_BULLET_SIZE: Vec2 = Vec2::new(8.0, 8.0);
pub const ENEMY_SCALE: f32 = 0.5;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct EnemyBullet;

#[derive(Clone, Copy, PartialEq, Eq, Component)]
pub enum EnemyType {
    Normal,
    Zigzag,
    Chaser,
}

impl EnemyType {
    pub fn initial_hp(self) -> u32 {
        match self {
            Self::Normal => 1,
            Self::Zigzag => 2,
            Self::Chaser => 3,
        }
    }

    pub fn score(self) -> u32 {
        match self {
            Self::Normal => 10,
            Self::Zigzag => 20,
            Self::Chaser => 30,
        }
    }

    pub fn spawn_weight(self, level: u32) -> u32 {
        if level < 2 {
            return u32::from(matches!(self, Self::Normal));
        }

        match self {
            Self::Normal => 5,
            Self::Zigzag => 3,
            Self::Chaser => 2,
        }
    }
}

#[derive(Component)]
pub struct ZigzagMotion {
    pub phase: f32,
}

#[derive(Component)]
pub struct ChasePlayer;

#[derive(Component)]
pub struct EnemyHitFlash(pub Timer);

impl EnemyHitFlash {
    pub fn new() -> Self {
        Self(Timer::from_seconds(0.06, TimerMode::Once))
    }
}

#[derive(Component)]
pub struct EnemyFireTimer(pub Timer);

#[derive(Resource)]
pub struct SpawnState {
    pub timer: Timer,
    pub current_interval: f32,
}

impl Default for SpawnState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            current_interval: 1.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct Difficulty {
    pub level: u32,
    pub elapsed_time: f32,
}

#[derive(Bundle)]
pub struct EnemyBundle {
    enemy: Enemy,
    enemy_type: EnemyType,
    health: Health,
    sprite: Sprite,
    transform: Transform,
    velocity: Velocity,
    collider: Collider,
    fire_timer: EnemyFireTimer,
    offscreen: OffscreenDespawn,
    cleanup: InGameEntity,
}

impl EnemyBundle {
    pub fn new(
        enemy_type: EnemyType,
        game_assets: &GameAssets,
        bounds: &GameBounds,
        y: f32,
        speed_multiplier: f32,
        fire_interval: f32,
    ) -> Self {
        Self {
            enemy: Enemy,
            enemy_type,
            health: Health::new(enemy_type.initial_hp()),
            sprite: Sprite::from_image(game_assets.asteroid()),
            transform: Transform::from_xyz(bounds.spawn_x(), y, layer::ENEMY)
                .with_scale(Vec3::splat(ENEMY_SCALE)),
            velocity: Velocity(Vec2::new(-ENEMY_SPEED * speed_multiplier, 0.0)),
            collider: Collider {
                size: ENEMY_SIZE * ENEMY_SCALE,
            },
            fire_timer: EnemyFireTimer(Timer::from_seconds(fire_interval, TimerMode::Repeating)),
            offscreen: OffscreenDespawn::horizontal(120.0),
            cleanup: InGameEntity,
        }
    }
}

#[derive(Bundle)]
pub struct EnemyBulletBundle {
    bullet: EnemyBullet,
    sprite: Sprite,
    transform: Transform,
    velocity: Velocity,
    collider: Collider,
    lifetime: Lifetime,
    offscreen: OffscreenDespawn,
    cleanup: InGameEntity,
}

impl EnemyBulletBundle {
    pub fn new(position: Vec3, velocity: Vec2) -> Self {
        Self {
            bullet: EnemyBullet,
            sprite: Sprite::from_color(Color::srgb(1.0, 0.3, 0.3), ENEMY_BULLET_SIZE),
            transform: Transform::from_xyz(position.x, position.y, layer::BULLET),
            velocity: Velocity(velocity),
            collider: Collider {
                size: ENEMY_BULLET_SIZE,
            },
            lifetime: Lifetime(Timer::from_seconds(
                BULLET_LIFETIME_SECONDS,
                TimerMode::Once,
            )),
            offscreen: OffscreenDespawn::new(Vec2::splat(120.0)),
            cleanup: InGameEntity,
        }
    }
}
