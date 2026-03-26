use bevy::prelude::*;
use std::time::Duration;

use super::assets::GameAssets;
use super::core::{
    Collider, GameBounds, Health, InGameEntity, Lifetime, OffscreenDespawn, Velocity, frand_range,
    layer,
};
use super::player::Player;
use super::state::{GameState, PlayState};
use super::{GameplaySet, SimulationSet};

const ENEMY_SPEED: f32 = 300.0;
const ENEMY_BULLET_SPEED: f32 = 400.0;
const ENEMY_FIRE_INTERVAL: f32 = 2.0;
const SPAWN_INTERVAL_SECONDS: f32 = 1.0;
const DIFFICULTY_INTERVAL: f32 = 15.0;
const BULLET_LIFETIME_SECONDS: f32 = 3.0;

pub(crate) const ENEMY_SIZE: Vec2 = Vec2::new(30.0, 30.0);
pub(crate) const ENEMY_BULLET_SIZE: Vec2 = Vec2::new(8.0, 8.0);
pub(crate) const ENEMY_SCALE: f32 = 0.5;

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
}

#[derive(Component)]
pub struct ZigzagPhase(pub f32);

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
            timer: Timer::from_seconds(SPAWN_INTERVAL_SECONDS, TimerMode::Repeating),
            current_interval: SPAWN_INTERVAL_SECONDS,
        }
    }
}

#[derive(Resource, Default)]
pub struct Difficulty {
    pub level: u32,
    pub elapsed_time: f32,
}

#[derive(Bundle)]
struct EnemyBundle {
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
    fn new(
        enemy_type: EnemyType,
        game_assets: &GameAssets,
        bounds: &GameBounds,
        y: f32,
        level: u32,
    ) -> Self {
        Self {
            enemy: Enemy,
            enemy_type,
            health: Health::new(enemy_type.initial_hp()),
            sprite: Sprite::from_image(game_assets.asteroid()),
            transform: Transform::from_xyz(bounds.spawn_x(), y, layer::ENEMY)
                .with_scale(Vec3::splat(ENEMY_SCALE)),
            velocity: Velocity(Vec2::new(
                -ENEMY_SPEED * speed_multiplier_for_level(level),
                0.0,
            )),
            collider: Collider {
                size: ENEMY_SIZE * ENEMY_SCALE,
            },
            fire_timer: EnemyFireTimer(Timer::from_seconds(
                enemy_fire_interval_for_level(level),
                TimerMode::Repeating,
            )),
            offscreen: OffscreenDespawn::horizontal(120.0),
            cleanup: InGameEntity,
        }
    }
}

#[derive(Bundle)]
struct EnemyBulletBundle {
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
    fn new(position: Vec3, velocity: Vec2) -> Self {
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

type EnemyVelocityQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static mut Velocity,
        &'static EnemyType,
        Option<&'static ZigzagPhase>,
    ),
    With<Enemy>,
>;

type EnemyVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Ref<'static, Health>,
        Option<&'static mut EnemyHitFlash>,
        &'static mut Sprite,
    ),
    With<Enemy>,
>;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnState>()
            .init_resource::<Difficulty>()
            .add_systems(OnEnter(GameState::InGame), reset_enemy_progress)
            .add_systems(
                Update,
                (
                    update_difficulty,
                    enemy_spawner,
                    update_enemy_velocity,
                    enemy_fire_system,
                )
                    .chain()
                    .in_set(SimulationSet::Prepare)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            )
            .add_systems(
                Update,
                clamp_enemy_position
                    .in_set(SimulationSet::PostMove)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            )
            .add_systems(
                Update,
                update_enemy_visuals
                    .in_set(GameplaySet::Fx)
                    .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
            );
    }
}

pub fn spawn_interval_for_level(level: u32) -> f32 {
    (SPAWN_INTERVAL_SECONDS - level as f32 * 0.15).max(0.3)
}

pub fn enemy_fire_interval_for_level(level: u32) -> f32 {
    (ENEMY_FIRE_INTERVAL - level as f32 * 0.2).max(0.5)
}

fn speed_multiplier_for_level(level: u32) -> f32 {
    (1.0 + level as f32 * 0.1).min(2.0)
}

fn random_enemy_type(level: u32) -> EnemyType {
    if level < 2 {
        return EnemyType::Normal;
    }

    match fastrand::u32(0..10) {
        0..=4 => EnemyType::Normal,
        5..=7 => EnemyType::Zigzag,
        _ => EnemyType::Chaser,
    }
}

fn reset_enemy_progress(mut spawn: ResMut<SpawnState>, mut difficulty: ResMut<Difficulty>) {
    *spawn = SpawnState::default();
    *difficulty = Difficulty::default();
}

fn update_difficulty(time: Res<Time>, mut difficulty: ResMut<Difficulty>) {
    difficulty.elapsed_time += time.delta_secs();
    let next_level = (difficulty.elapsed_time / DIFFICULTY_INTERVAL) as u32;

    if next_level > difficulty.level {
        difficulty.level = next_level;
        crate::dlog!("Difficulty increased to level {}", difficulty.level);
    }
}

fn enemy_spawner(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn: ResMut<SpawnState>,
    bounds: Res<GameBounds>,
    game_assets: Res<GameAssets>,
    difficulty: Res<Difficulty>,
) {
    let new_interval = spawn_interval_for_level(difficulty.level);
    if (new_interval - spawn.current_interval).abs() > f32::EPSILON {
        let old_duration = spawn.timer.duration().as_secs_f32().max(0.0001);
        let fraction = (spawn.timer.elapsed_secs() / old_duration).clamp(0.0, 1.0);

        spawn.current_interval = new_interval;
        spawn
            .timer
            .set_duration(Duration::from_secs_f32(new_interval));
        spawn
            .timer
            .set_elapsed(Duration::from_secs_f32(fraction * new_interval));

        crate::dlog!("Spawn interval changed to {:.2}s", new_interval);
    }

    spawn.timer.tick(time.delta());
    let spawn_count = spawn.timer.times_finished_this_tick().min(3);
    for _ in 0..spawn_count {
        let enemy_type = random_enemy_type(difficulty.level);
        let y = frand_range(bounds.spawn_y_range());
        let mut enemy = commands.spawn(EnemyBundle::new(
            enemy_type,
            &game_assets,
            &bounds,
            y,
            difficulty.level,
        ));

        if enemy_type == EnemyType::Zigzag {
            enemy.insert(ZigzagPhase(fastrand::f32() * std::f32::consts::TAU));
        }
    }
}

fn update_enemy_velocity(
    time: Res<Time>,
    mut query: EnemyVelocityQuery,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    let player_y = player_query
        .single()
        .map(|transform| transform.translation.y)
        .unwrap_or(0.0);

    for (transform, mut velocity, enemy_type, phase) in &mut query {
        match enemy_type {
            EnemyType::Normal => {
                velocity.0.y = 0.0;
            }
            EnemyType::Zigzag => {
                let offset = phase.map_or(0.0, |phase| phase.0);
                velocity.0.y =
                    (time.elapsed_secs() * 4.0 + transform.translation.x * 0.01 + offset).sin()
                        * 150.0;
            }
            EnemyType::Chaser => {
                let diff = player_y - transform.translation.y;
                velocity.0.y = diff.clamp(-120.0, 120.0);
            }
        }
    }
}

fn clamp_enemy_position(bounds: Res<GameBounds>, mut query: Query<&mut Transform, With<Enemy>>) {
    let (y_min, y_max) = bounds.player_y_range(20.0);

    for mut transform in &mut query {
        transform.translation.y = transform.translation.y.clamp(y_min, y_max);
    }
}

fn enemy_fire_system(
    mut commands: Commands,
    time: Res<Time>,
    mut enemy_query: Query<(&Transform, &mut EnemyFireTimer), With<Enemy>>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    let player_pos = player_query
        .single()
        .map(|transform| transform.translation)
        .unwrap_or(Vec3::ZERO);

    for (transform, mut fire_timer) in &mut enemy_query {
        fire_timer.0.tick(time.delta());
        let fire_count = fire_timer.0.times_finished_this_tick().min(2);

        for _ in 0..fire_count {
            let delta = (player_pos - transform.translation).truncate();
            let direction = if delta.length_squared() > 1e-6 {
                delta.normalize()
            } else {
                -Vec2::X
            };

            commands.spawn(EnemyBulletBundle::new(
                transform.translation,
                direction * ENEMY_BULLET_SPEED,
            ));
        }
    }
}

fn update_enemy_visuals(mut commands: Commands, time: Res<Time>, mut query: EnemyVisualQuery) {
    for (entity, health, flash, mut sprite) in &mut query {
        let mut should_restore_health_color = health.is_changed();

        if let Some(mut flash) = flash {
            flash.0.tick(time.delta());
            if !flash.0.is_finished() {
                sprite.color = Color::WHITE;
                continue;
            }

            commands.entity(entity).remove::<EnemyHitFlash>();
            should_restore_health_color = true;
        }

        if should_restore_health_color {
            let ratio = (health.current as f32 / health.max.max(1) as f32).clamp(0.0, 1.0);
            sprite.color = Color::srgb(1.0, ratio, ratio);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enemy_type_descriptors_match_expected_balance() {
        assert_eq!(EnemyType::Normal.initial_hp(), 1);
        assert_eq!(EnemyType::Zigzag.initial_hp(), 2);
        assert_eq!(EnemyType::Chaser.initial_hp(), 3);
        assert_eq!(EnemyType::Normal.score(), 10);
        assert_eq!(EnemyType::Zigzag.score(), 20);
        assert_eq!(EnemyType::Chaser.score(), 30);
    }

    #[test]
    fn difficulty_helpers_clamp_to_limits() {
        assert_eq!(spawn_interval_for_level(0), 1.0);
        assert_eq!(spawn_interval_for_level(10), 0.3);
        assert_eq!(enemy_fire_interval_for_level(0), 2.0);
        assert_eq!(enemy_fire_interval_for_level(10), 0.5);
    }
}
