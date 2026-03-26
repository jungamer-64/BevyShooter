use bevy::prelude::*;

use super::GameplaySet;
use super::combat::{EnemyDestroyedEvent, EnemyHitEvent, PlayerDamagedEvent};
use super::core::{InGameEntity, Lifetime, MainCamera, Velocity, frand_range, layer};
use super::state::{GameState, PlayState};

const EXPLOSION_DURATION: f32 = 0.5;

#[derive(Component)]
struct ExplosionParticle;

#[derive(Component)]
pub(crate) struct CameraShake {
    timer: Timer,
    magnitude: f32,
}

#[derive(Bundle)]
struct ExplosionParticleBundle {
    sprite: Sprite,
    transform: Transform,
    velocity: Velocity,
    lifetime: Lifetime,
    particle: ExplosionParticle,
    cleanup: InGameEntity,
}

impl ExplosionParticleBundle {
    fn new(position: Vec3, velocity: Vec2, size: f32, color: Color) -> Self {
        Self {
            sprite: Sprite::from_color(color, Vec2::splat(size)),
            transform: Transform::from_xyz(position.x, position.y, layer::FX),
            velocity: Velocity(velocity),
            lifetime: Lifetime(Timer::from_seconds(EXPLOSION_DURATION, TimerMode::Once)),
            particle: ExplosionParticle,
            cleanup: InGameEntity,
        }
    }
}

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_enemy_hit)
            .add_observer(on_enemy_destroyed)
            .add_observer(on_player_damaged)
            .add_systems(OnExit(GameState::InGame), reset_camera_shake)
            .add_systems(OnEnter(PlayState::Paused), reset_camera_shake)
            .add_systems(
                Update,
                (apply_camera_shake, fade_explosions)
                    .chain()
                    .in_set(GameplaySet::Fx),
            );
    }
}

pub fn spawn_explosion(commands: &mut Commands, position: Vec3) {
    for _ in 0..12 {
        let angle = frand_range(0.0..std::f32::consts::TAU);
        let speed = frand_range(100.0..300.0);
        let size = frand_range(3.0..8.0);
        let green = frand_range(0.3..0.8);
        let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

        commands.spawn(ExplosionParticleBundle::new(
            position,
            velocity,
            size,
            Color::srgb(1.0, green, 0.0),
        ));
    }
}

fn apply_camera_shake(
    mut commands: Commands,
    time: Res<Time>,
    camera: Single<(Entity, &mut Transform, &MainCamera, Option<&mut CameraShake>), With<MainCamera>>,
) {
    let (entity, mut transform, camera, shake) = camera.into_inner();
    let Some(mut shake) = shake else {
        return;
    };

    shake.timer.tick(time.delta());

    let duration = shake.timer.duration().as_secs_f32().max(0.0001);
    let t = 1.0 - (shake.timer.elapsed_secs() / duration).clamp(0.0, 1.0);
    let dx = frand_range(-1.0..1.0) * shake.magnitude * t;
    let dy = frand_range(-1.0..1.0) * shake.magnitude * t;

    transform.translation = camera.base + Vec3::new(dx, dy, 0.0);

    if shake.timer.is_finished() {
        transform.translation = camera.base;
        commands.entity(entity).remove::<CameraShake>();
    }
}

pub fn reset_camera_shake(
    mut commands: Commands,
    camera: Single<(Entity, &mut Transform, &MainCamera, Option<&CameraShake>), With<MainCamera>>,
) {
    let (entity, mut transform, camera, shake) = camera.into_inner();
    if shake.is_some() {
        transform.translation = camera.base;
        commands.entity(entity).remove::<CameraShake>();
    }
}

fn fade_explosions(mut query: Query<(&Lifetime, &mut Sprite), With<ExplosionParticle>>) {
    for (lifetime, mut sprite) in &mut query {
        let duration = lifetime.0.duration().as_secs_f32().max(0.0001);
        let alpha = (1.0 - lifetime.0.elapsed_secs() / duration).clamp(0.0, 1.0);
        sprite.color = sprite.color.with_alpha(alpha);
    }
}

fn on_enemy_hit(
    _event: On<EnemyHitEvent>,
    mut commands: Commands,
    camera: Single<(Entity, &Transform, &mut MainCamera, Option<&mut CameraShake>), With<MainCamera>>,
) {
    queue_camera_shake(&mut commands, camera.into_inner(), 2.0, 0.03);
}

fn on_enemy_destroyed(
    event: On<EnemyDestroyedEvent>,
    mut commands: Commands,
    camera: Single<(Entity, &Transform, &mut MainCamera, Option<&mut CameraShake>), With<MainCamera>>,
) {
    spawn_explosion(&mut commands, event.position);
    queue_camera_shake(&mut commands, camera.into_inner(), 4.0, 0.06);
}

fn on_player_damaged(
    event: On<PlayerDamagedEvent>,
    mut commands: Commands,
    camera: Single<(Entity, &Transform, &mut MainCamera, Option<&mut CameraShake>), With<MainCamera>>,
) {
    let (magnitude, duration) = if event.defeated {
        (20.0, 0.3)
    } else {
        (14.0, 0.15)
    };

    queue_camera_shake(&mut commands, camera.into_inner(), magnitude, duration);
}

fn queue_camera_shake(
    commands: &mut Commands,
    (entity, transform, mut camera, shake): (
        Entity,
        &Transform,
        Mut<MainCamera>,
        Option<Mut<CameraShake>>,
    ),
    magnitude: f32,
    duration: f32,
) {
    if let Some(mut shake) = shake {
        shake.magnitude = shake.magnitude.max(magnitude);
        let extended_duration = shake.timer.duration().as_secs_f32().max(duration);
        shake
            .timer
            .set_duration(std::time::Duration::from_secs_f32(extended_duration));
        shake.timer.reset();
        return;
    }

    camera.base = transform.translation;
    commands.entity(entity).insert(CameraShake {
        timer: Timer::from_seconds(duration, TimerMode::Once),
        magnitude,
    });
}
