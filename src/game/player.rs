use bevy::prelude::*;
use std::time::Duration;

use super::GameplaySet;
use super::assets::GameAssets;
use super::combat::{HitList, Pierce};
use super::shared::{
    Collider, GameBounds, GameEntity, Health, Lifetime, Velocity, capped_delta_seconds, layer,
    ready_once_timer,
};
use super::state::{GameState, PlayState};

const PLAYER_SPEED: f32 = 500.0;
const BULLET_SPEED: f32 = 800.0;
const FIRE_COOLDOWN_SECONDS: f32 = 0.2;
const BULLET_LIFETIME_SECONDS: f32 = 3.0;
const POWERUP_SPEED: f32 = 100.0;
const POWERUP_TRIPLE_DURATION: f32 = 10.0;
const POWERUP_RAPID_DURATION: f32 = 8.0;
const POWERUP_PIERCE_DURATION: f32 = 12.0;
const RAPID_FIRE_BONUS: f32 = 0.4;
const POWERUP_SIZE: Vec2 = Vec2::new(20.0, 20.0);

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
pub struct FireCooldown(pub Timer);

#[derive(Component, Default)]
pub struct PlayerStats {
    pub weapon_level: u32,
}

#[derive(Component)]
pub struct TripleShot(pub Timer);

impl TripleShot {
    pub fn new() -> Self {
        Self(Timer::from_seconds(
            POWERUP_TRIPLE_DURATION,
            TimerMode::Once,
        ))
    }

    pub fn remaining_secs(&self) -> f32 {
        (self.0.duration().as_secs_f32() - self.0.elapsed_secs()).max(0.0)
    }
}

#[derive(Component)]
pub struct RapidFire(pub Timer);

impl RapidFire {
    pub fn new() -> Self {
        Self(Timer::from_seconds(POWERUP_RAPID_DURATION, TimerMode::Once))
    }

    pub fn remaining_secs(&self) -> f32 {
        (self.0.duration().as_secs_f32() - self.0.elapsed_secs()).max(0.0)
    }
}

#[derive(Component)]
pub struct PierceShot(pub Timer);

impl PierceShot {
    pub fn new() -> Self {
        Self(Timer::from_seconds(
            POWERUP_PIERCE_DURATION,
            TimerMode::Once,
        ))
    }

    pub fn remaining_secs(&self) -> f32 {
        (self.0.duration().as_secs_f32() - self.0.elapsed_secs()).max(0.0)
    }
}

#[derive(Component)]
pub struct Invincible(pub Timer);

impl Invincible {
    pub fn new() -> Self {
        Self(Timer::from_seconds(INVINCIBILITY_SECONDS, TimerMode::Once))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PowerUpType {
    TripleShot,
    RapidFire,
    Shield,
    PierceShot,
}

#[derive(Component)]
pub struct PowerUpItem(pub PowerUpType);

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    sprite: Sprite,
    transform: Transform,
    collider: Collider,
    health: Health,
    cooldown: FireCooldown,
    stats: PlayerStats,
    cleanup: GameEntity,
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
            health: Health {
                current: PLAYER_MAX_HP,
                max: PLAYER_MAX_HP,
            },
            cooldown: FireCooldown(ready_once_timer(FIRE_COOLDOWN_SECONDS)),
            stats: PlayerStats::default(),
            cleanup: GameEntity,
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
    cleanup: GameEntity,
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
            cleanup: GameEntity,
        }
    }
}

#[derive(Bundle)]
struct PowerUpBundle {
    item: PowerUpItem,
    sprite: Sprite,
    transform: Transform,
    velocity: Velocity,
    collider: Collider,
    cleanup: GameEntity,
}

impl PowerUpBundle {
    fn new(position: Vec3, power_type: PowerUpType) -> Self {
        let color = match power_type {
            PowerUpType::TripleShot => Color::srgb(0.2, 0.6, 1.0),
            PowerUpType::RapidFire => Color::srgb(1.0, 1.0, 0.2),
            PowerUpType::PierceShot => Color::srgb(0.8, 0.2, 1.0),
            PowerUpType::Shield => Color::srgb(0.2, 1.0, 0.2),
        };

        Self {
            item: PowerUpItem(power_type),
            sprite: Sprite::from_color(color, POWERUP_SIZE),
            transform: Transform::from_xyz(position.x, position.y, layer::POWERUP),
            velocity: Velocity(Vec2::new(-POWERUP_SPEED, 0.0)),
            collider: Collider { size: POWERUP_SIZE },
            cleanup: GameEntity,
        }
    }
}

type PlayerShootQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static mut FireCooldown,
        &'static PlayerStats,
        Option<&'static TripleShot>,
        Option<&'static RapidFire>,
        Option<&'static PierceShot>,
    ),
    With<Player>,
>;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_player)
            .add_systems(
                Update,
                (
                    update_fire_cooldown,
                    tick_triple_shot,
                    tick_rapid_fire,
                    tick_pierce_shot,
                    player_movement,
                )
                    .chain()
                    .in_set(GameplaySet::Input)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            )
            .add_systems(
                Update,
                (powerup_movement, powerup_collection, player_shoot)
                    .chain()
                    .in_set(GameplaySet::Spawn)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            )
            .add_systems(
                Update,
                bullet_movement
                    .in_set(GameplaySet::Movement)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

pub fn spawn_powerup(commands: &mut Commands, position: Vec3, power_type: PowerUpType) {
    commands.spawn(PowerUpBundle::new(position, power_type));
}

fn spawn_player(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(PlayerBundle::new(&game_assets));
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

    let (x_min, x_max) = bounds.player_x_range(20.0);
    let (y_min, y_max) = bounds.player_y_range(20.0);

    transform.translation.x = (transform.translation.x
        + direction.x * PLAYER_SPEED * time.delta_secs())
    .clamp(x_min, x_max);
    transform.translation.y = (transform.translation.y
        + direction.y * PLAYER_SPEED * time.delta_secs())
    .clamp(y_min, y_max);
}

fn update_fire_cooldown(time: Res<Time>, mut query: Query<&mut FireCooldown>) {
    for mut cooldown in &mut query {
        cooldown.0.tick(time.delta());
    }
}

fn tick_triple_shot(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut PlayerStats, &mut TripleShot), With<Player>>,
) {
    let Ok((entity, mut stats, mut effect)) = query.single_mut() else {
        return;
    };

    effect.0.tick(time.delta());
    if effect.0.is_finished() {
        stats.weapon_level = 0;
        commands.entity(entity).remove::<TripleShot>();
        crate::dlog!("Triple Shot expired!");
    }
}

fn tick_rapid_fire(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut RapidFire), With<Player>>,
) {
    let Ok((entity, mut effect)) = query.single_mut() else {
        return;
    };

    effect.0.tick(time.delta());
    if effect.0.is_finished() {
        commands.entity(entity).remove::<RapidFire>();
        crate::dlog!("Rapid Fire expired!");
    }
}

fn tick_pierce_shot(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut PierceShot), With<Player>>,
) {
    let Ok((entity, mut effect)) = query.single_mut() else {
        return;
    };

    effect.0.tick(time.delta());
    if effect.0.is_finished() {
        commands.entity(entity).remove::<PierceShot>();
        crate::dlog!("Pierce Shot expired!");
    }
}

fn player_shoot(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: PlayerShootQuery,
    game_assets: Res<GameAssets>,
) {
    let Ok((player_transform, mut cooldown, stats, triple, rapid, pierce_effect)) =
        query.single_mut()
    else {
        return;
    };

    if !keyboard_input.pressed(KeyCode::Space) || !cooldown.0.is_finished() {
        return;
    }

    let mut angles = [0.0_f32; 3];
    let spread_active = stats.weapon_level > 0 || triple.is_some();
    let shot_count = if spread_active {
        angles[1] = 15.0_f32.to_radians();
        angles[2] = -15.0_f32.to_radians();
        3
    } else {
        1
    };

    for &angle in &angles[..shot_count] {
        let velocity = Vec2::new(angle.cos(), angle.sin()) * BULLET_SPEED;
        let position = player_transform.translation;
        let mut bullet = commands.spawn(BulletBundle::new(&game_assets, position, velocity));

        if pierce_effect.is_some() {
            bullet.insert((Pierce(2), HitList::default()));
        }
    }

    let bonus = if rapid.is_some() {
        RAPID_FIRE_BONUS
    } else {
        0.0
    };
    let actual_cooldown = FIRE_COOLDOWN_SECONDS * (1.0 - bonus.clamp(0.0, 0.9));
    cooldown
        .0
        .set_duration(Duration::from_secs_f32(actual_cooldown));
    cooldown.0.reset();
}

fn bullet_movement(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity), With<Bullet>>) {
    let dt = capped_delta_seconds(&time);
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;
    }
}

fn powerup_movement(
    mut commands: Commands,
    time: Res<Time>,
    bounds: Res<GameBounds>,
    mut query: Query<(Entity, &mut Transform, &Velocity), With<PowerUpItem>>,
) {
    let dt = capped_delta_seconds(&time);
    for (entity, mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;

        if transform.translation.x < bounds.despawn_x() {
            commands.entity(entity).despawn();
        }
    }
}

fn powerup_collection(
    mut commands: Commands,
    powerup_query: Query<(Entity, &Transform, &Collider, &PowerUpItem)>,
    mut player_query: Query<
        (Entity, &Transform, &Collider, &mut PlayerStats, &mut Health),
        With<Player>,
    >,
) {
    let Ok((player_entity, player_tf, player_collider, mut stats, mut health)) =
        player_query.single_mut()
    else {
        return;
    };

    for (entity, transform, collider, item) in &powerup_query {
        if !super::combat::collide(
            player_tf.translation,
            player_collider.size,
            transform.translation,
            collider.size,
        ) {
            continue;
        }

        match item.0 {
            PowerUpType::TripleShot => {
                stats.weapon_level = 1;
                commands.entity(player_entity).insert(TripleShot::new());
                crate::dlog!("Power-up: Triple Shot! ({:.0}s)", POWERUP_TRIPLE_DURATION);
            }
            PowerUpType::RapidFire => {
                commands.entity(player_entity).insert(RapidFire::new());
                crate::dlog!("Power-up: Rapid Fire! ({:.0}s)", POWERUP_RAPID_DURATION);
            }
            PowerUpType::PierceShot => {
                commands.entity(player_entity).insert(PierceShot::new());
                crate::dlog!("Power-up: Pierce Shot! ({:.0}s)", POWERUP_PIERCE_DURATION);
            }
            PowerUpType::Shield => {
                health.current = (health.current + 1).min(health.max);
                crate::dlog!("Power-up: Shield! (HP: {})", health.current);
            }
        }

        commands.entity(entity).despawn();
    }
}
