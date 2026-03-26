use bevy::prelude::*;
use std::time::Duration;

use super::GameplaySet;
use super::assets::GameAssets;
use super::combat::{HitList, Pierce};
use super::core::{
    Collider, GameBounds, Health, InGameEntity, Lifetime, OffscreenDespawn, Velocity,
    capped_delta_seconds, layer, ready_once_timer, remaining_timer_secs,
};
use super::state::{GameState, PlayState};

const PLAYER_SPEED: f32 = 500.0;
const BULLET_SPEED: f32 = 800.0;
const FIRE_COOLDOWN_SECONDS: f32 = 0.2;
const BULLET_LIFETIME_SECONDS: f32 = 3.0;
const RAPID_FIRE_BONUS: f32 = 0.4;
const PIERCE_SHOT_CHARGES: u32 = 2;
const SPREAD_SHOT_ANGLES: [f32; 3] = [0.0, 15.0_f32.to_radians(), -15.0_f32.to_radians()];
const SINGLE_SHOT_ANGLES: [f32; 1] = [0.0];

pub const PLAYER_MAX_HP: u32 = 3;
pub(crate) const PLAYER_SIZE: Vec2 = Vec2::new(30.0, 30.0);
pub(crate) const BULLET_SIZE: Vec2 = Vec2::new(10.0, 5.0);
pub(crate) const PLAYER_SCALE: f32 = 0.5;
pub(crate) const BULLET_SCALE: f32 = 0.3;
pub(crate) const INVINCIBILITY_SECONDS: f32 = 1.5;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Bullet;

#[derive(Component)]
pub struct PlayerWeapons {
    pub fire_cooldown: Timer,
    pub triple_shot: Option<Timer>,
    pub rapid_fire: Option<Timer>,
    pub pierce_shot: Option<Timer>,
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
            triple_shot: None,
            rapid_fire: None,
            pierce_shot: None,
        }
    }

    pub fn tick(&mut self, delta: Duration) -> WeaponExpiry {
        self.fire_cooldown.tick(delta);

        WeaponExpiry {
            triple_shot: tick_effect(&mut self.triple_shot, delta),
            rapid_fire: tick_effect(&mut self.rapid_fire, delta),
            pierce_shot: tick_effect(&mut self.pierce_shot, delta),
        }
    }

    pub fn ready_to_fire(&self) -> bool {
        self.fire_cooldown.is_finished()
    }

    pub fn fire_angles(&self) -> &'static [f32] {
        if self.triple_shot.is_some() {
            &SPREAD_SHOT_ANGLES
        } else {
            &SINGLE_SHOT_ANGLES
        }
    }

    pub fn has_pierce_shot(&self) -> bool {
        self.pierce_shot.is_some()
    }

    pub fn reset_fire_cooldown(&mut self) {
        let bonus = if self.rapid_fire.is_some() {
            RAPID_FIRE_BONUS
        } else {
            0.0
        };
        let cooldown = FIRE_COOLDOWN_SECONDS * (1.0 - bonus.clamp(0.0, 0.9));

        self.fire_cooldown
            .set_duration(Duration::from_secs_f32(cooldown));
        self.fire_cooldown.reset();
    }

    pub fn activate_triple_shot(&mut self, seconds: f32) {
        self.triple_shot = Some(Timer::from_seconds(seconds, TimerMode::Once));
    }

    pub fn activate_rapid_fire(&mut self, seconds: f32) {
        self.rapid_fire = Some(Timer::from_seconds(seconds, TimerMode::Once));
    }

    pub fn activate_pierce_shot(&mut self, seconds: f32) {
        self.pierce_shot = Some(Timer::from_seconds(seconds, TimerMode::Once));
    }

    pub fn remaining_triple_shot(&self) -> Option<f32> {
        self.triple_shot
            .as_ref()
            .map(remaining_timer_secs)
            .filter(|remaining| *remaining > 0.0)
    }

    pub fn remaining_rapid_fire(&self) -> Option<f32> {
        self.rapid_fire
            .as_ref()
            .map(remaining_timer_secs)
            .filter(|remaining| *remaining > 0.0)
    }

    pub fn remaining_pierce_shot(&self) -> Option<f32> {
        self.pierce_shot
            .as_ref()
            .map(remaining_timer_secs)
            .filter(|remaining| *remaining > 0.0)
    }
}

#[derive(Default)]
pub struct WeaponExpiry {
    pub triple_shot: bool,
    pub rapid_fire: bool,
    pub pierce_shot: bool,
}

#[derive(Component, Default)]
pub struct PlayerStatus {
    pub invincible: Option<Timer>,
}

impl PlayerStatus {
    pub fn is_invincible(&self) -> bool {
        self.invincible.is_some()
    }

    pub fn grant_invincibility(&mut self, seconds: f32) {
        self.invincible = Some(Timer::from_seconds(seconds, TimerMode::Once));
    }

    pub fn remaining_invincibility(&self) -> Option<f32> {
        self.invincible
            .as_ref()
            .map(remaining_timer_secs)
            .filter(|remaining| *remaining > 0.0)
    }
}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    sprite: Sprite,
    transform: Transform,
    collider: Collider,
    health: Health,
    weapons: PlayerWeapons,
    status: PlayerStatus,
    cleanup: InGameEntity,
}

impl PlayerBundle {
    fn new(game_assets: &GameAssets) -> Self {
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
            status: PlayerStatus::default(),
            cleanup: InGameEntity,
        }
    }
}

#[derive(Bundle)]
struct BulletBundle {
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
    fn new(game_assets: &GameAssets, position: Vec3, velocity: Vec2) -> Self {
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

type PlayerShootQuery<'w, 's> =
    Query<'w, 's, (&'static Transform, &'static mut PlayerWeapons), With<Player>>;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_player)
            .add_systems(
                Update,
                (tick_player_weapons, player_movement, player_shoot)
                    .chain()
                    .in_set(GameplaySet::Input)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

fn spawn_player(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(PlayerBundle::new(&game_assets));
}

fn tick_player_weapons(time: Res<Time>, mut query: Query<&mut PlayerWeapons, With<Player>>) {
    let Ok(mut weapons) = query.single_mut() else {
        return;
    };

    let expired = weapons.tick(time.delta());
    if expired.triple_shot {
        crate::dlog!("Triple Shot expired!");
    }
    if expired.rapid_fire {
        crate::dlog!("Rapid Fire expired!");
    }
    if expired.pierce_shot {
        crate::dlog!("Pierce Shot expired!");
    }
}

fn player_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    bounds: Res<GameBounds>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    let mut direction = Vec2::ZERO;
    if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
    }

    let dt = capped_delta_seconds(&time);
    let (x_min, x_max) = bounds.player_x_range(20.0);
    let (y_min, y_max) = bounds.player_y_range(20.0);

    transform.translation.x =
        (transform.translation.x + direction.x * PLAYER_SPEED * dt).clamp(x_min, x_max);
    transform.translation.y =
        (transform.translation.y + direction.y * PLAYER_SPEED * dt).clamp(y_min, y_max);
}

fn player_shoot(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: PlayerShootQuery,
    game_assets: Res<GameAssets>,
) {
    let Ok((player_transform, mut weapons)) = query.single_mut() else {
        return;
    };

    if !keyboard_input.pressed(KeyCode::Space) || !weapons.ready_to_fire() {
        return;
    }

    for &angle in weapons.fire_angles() {
        let velocity = Vec2::new(angle.cos(), angle.sin()) * BULLET_SPEED;
        let mut bullet = commands.spawn(BulletBundle::new(
            &game_assets,
            player_transform.translation,
            velocity,
        ));

        if weapons.has_pierce_shot() {
            bullet.insert((Pierce(PIERCE_SHOT_CHARGES), HitList::default()));
        }
    }

    weapons.reset_fire_cooldown();
}

fn tick_effect(timer: &mut Option<Timer>, delta: Duration) -> bool {
    let Some(effect) = timer.as_mut() else {
        return false;
    };

    effect.tick(delta);
    if effect.is_finished() {
        *timer = None;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_weapons_expire_cleanly() {
        let mut weapons = PlayerWeapons::new();
        weapons.activate_triple_shot(1.0);
        weapons.activate_rapid_fire(1.0);
        weapons.activate_pierce_shot(1.0);

        let expired = weapons.tick(Duration::from_secs_f32(1.5));

        assert!(expired.triple_shot);
        assert!(expired.rapid_fire);
        assert!(expired.pierce_shot);
        assert!(weapons.remaining_triple_shot().is_none());
        assert!(weapons.remaining_rapid_fire().is_none());
        assert!(weapons.remaining_pierce_shot().is_none());
    }

    #[test]
    fn player_status_reports_invincibility() {
        let mut status = PlayerStatus::default();
        assert!(!status.is_invincible());

        status.grant_invincibility(INVINCIBILITY_SECONDS);

        assert!(status.is_invincible());
        assert!(status.remaining_invincibility().is_some());
    }
}
