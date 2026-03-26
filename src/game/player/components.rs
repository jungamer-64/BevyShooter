use bevy::prelude::*;
use std::time::Duration;

use super::super::assets::GameAssets;
use super::super::core::{
    Collider, Health, InGameEntity, Lifetime, OffscreenDespawn, Velocity, layer, ready_once_timer,
    remaining_timer_secs,
};

const FIRE_COOLDOWN_SECONDS: f32 = 0.2;
const BULLET_LIFETIME_SECONDS: f32 = 3.0;
const RAPID_FIRE_BONUS: f32 = 0.4;

pub(crate) const BULLET_SPEED: f32 = 800.0;
pub(crate) const SPREAD_SHOT_ANGLES: [f32; 3] =
    [0.0, 15.0_f32.to_radians(), -15.0_f32.to_radians()];
pub(crate) const SINGLE_SHOT_ANGLES: [f32; 1] = [0.0];

pub const PLAYER_MAX_HP: u32 = 3;
pub const PLAYER_SIZE: Vec2 = Vec2::new(30.0, 30.0);
pub const BULLET_SIZE: Vec2 = Vec2::new(10.0, 5.0);
pub const PLAYER_SCALE: f32 = 0.5;
pub const BULLET_SCALE: f32 = 0.3;
pub const INVINCIBILITY_SECONDS: f32 = 1.5;
pub const PIERCE_SHOT_CHARGES: u32 = 2;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Bullet;

#[derive(Component, Default)]
pub struct PlayerIntent {
    pub move_dir: Vec2,
    pub firing: bool,
}

#[derive(Component)]
pub struct PlayerWeapons {
    pub fire_cooldown: Timer,
}

impl Default for PlayerWeapons {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerWeapons {
    pub fn new() -> Self {
        Self {
            fire_cooldown: ready_once_timer(FIRE_COOLDOWN_SECONDS),
        }
    }

    pub fn tick_cooldown(&mut self, delta: Duration) {
        self.fire_cooldown.tick(delta);
    }

    pub fn ready_to_fire(&self) -> bool {
        self.fire_cooldown.is_finished()
    }

    pub fn reset_fire_cooldown(&mut self, rapid_fire_active: bool) {
        let bonus = if rapid_fire_active {
            RAPID_FIRE_BONUS
        } else {
            0.0
        };
        let cooldown = FIRE_COOLDOWN_SECONDS * (1.0 - bonus.clamp(0.0, 0.9));

        self.fire_cooldown
            .set_duration(Duration::from_secs_f32(cooldown));
        self.fire_cooldown.reset();
    }
}

pub trait TimedEffectComponent {
    fn new(seconds: f32) -> Self;
    fn timer(&self) -> &Timer;
    fn timer_mut(&mut self) -> &mut Timer;

    fn remaining_secs(&self) -> f32 {
        remaining_timer_secs(self.timer())
    }

    fn elapsed_secs(&self) -> f32 {
        self.timer().elapsed_secs()
    }

    fn tick(&mut self, delta: Duration) {
        self.timer_mut().tick(delta);
    }

    fn is_finished(&self) -> bool {
        self.timer().is_finished()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InvincibilitySnapshot {
    pub remaining_secs: f32,
    pub elapsed_secs: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerEffectSnapshot {
    pub triple_shot: Option<f32>,
    pub rapid_fire: Option<f32>,
    pub pierce_shot: Option<f32>,
    pub invincible: Option<InvincibilitySnapshot>,
}

impl PlayerEffectSnapshot {
    pub fn from_components(
        triple_shot: Option<&TripleShot>,
        rapid_fire: Option<&RapidFire>,
        pierce_shot: Option<&PierceShot>,
        invincible: Option<&Invincible>,
    ) -> Self {
        Self {
            triple_shot: Self::remaining(triple_shot),
            rapid_fire: Self::remaining(rapid_fire),
            pierce_shot: Self::remaining(pierce_shot),
            invincible: invincible.map(|effect| InvincibilitySnapshot {
                remaining_secs: effect.remaining_secs(),
                elapsed_secs: effect.elapsed_secs(),
            }),
        }
    }

    pub fn has_triple_shot(&self) -> bool {
        self.triple_shot.is_some()
    }

    pub fn has_rapid_fire(&self) -> bool {
        self.rapid_fire.is_some()
    }

    pub fn has_pierce_shot(&self) -> bool {
        self.pierce_shot.is_some()
    }

    pub fn is_invincible(&self) -> bool {
        self.invincible.is_some()
    }

    pub fn invincible_visible(&self) -> bool {
        self.invincible
            .map(|effect| (effect.elapsed_secs * 10.0) as i32 % 2 == 0)
            .unwrap_or(true)
    }

    fn remaining<T: TimedEffectComponent>(effect: Option<&T>) -> Option<f32> {
        effect.map(TimedEffectComponent::remaining_secs)
    }
}

macro_rules! timed_effect_component {
    ($name:ident) => {
        #[derive(Component, Debug)]
        pub struct $name(pub Timer);

        impl TimedEffectComponent for $name {
            fn new(seconds: f32) -> Self {
                Self(Timer::from_seconds(seconds, TimerMode::Once))
            }

            fn timer(&self) -> &Timer {
                &self.0
            }

            fn timer_mut(&mut self) -> &mut Timer {
                &mut self.0
            }
        }

        impl $name {
            pub fn new(seconds: f32) -> Self {
                <Self as TimedEffectComponent>::new(seconds)
            }

            #[allow(dead_code)]
            pub fn remaining_secs(&self) -> f32 {
                <Self as TimedEffectComponent>::remaining_secs(self)
            }
        }
    };
}

timed_effect_component!(TripleShot);
timed_effect_component!(RapidFire);
timed_effect_component!(PierceShot);
timed_effect_component!(Invincible);

impl Invincible {
    pub fn elapsed_secs(&self) -> f32 {
        <Self as TimedEffectComponent>::elapsed_secs(self)
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    sprite: Sprite,
    transform: Transform,
    collider: Collider,
    health: Health,
    weapons: PlayerWeapons,
    intent: PlayerIntent,
    cleanup: InGameEntity,
}

impl PlayerBundle {
    pub fn new(game_assets: &GameAssets) -> Self {
        Self {
            player: Player,
            sprite: Sprite::from_image(game_assets.spaceship()),
            transform: Transform::from_xyz(-300.0, 0.0, layer::PLAYER)
                .with_scale(Vec3::splat(PLAYER_SCALE)),
            collider: Collider {
                size: PLAYER_SIZE * PLAYER_SCALE,
            },
            health: Health::new(PLAYER_MAX_HP),
            weapons: PlayerWeapons::new(),
            intent: PlayerIntent::default(),
            cleanup: InGameEntity,
        }
    }
}

#[derive(Bundle)]
pub struct BulletBundle {
    bullet: Bullet,
    sprite: Sprite,
    transform: Transform,
    velocity: Velocity,
    collider: Collider,
    lifetime: Lifetime,
    offscreen: OffscreenDespawn,
    cleanup: InGameEntity,
}

impl BulletBundle {
    pub fn new(game_assets: &GameAssets, position: Vec3, velocity: Vec2) -> Self {
        Self {
            bullet: Bullet,
            sprite: Sprite::from_image(game_assets.bullet()),
            transform: Transform::from_xyz(position.x, position.y, layer::BULLET)
                .with_scale(Vec3::splat(BULLET_SCALE)),
            velocity: Velocity(velocity),
            collider: Collider {
                size: BULLET_SIZE * BULLET_SCALE,
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
