use bevy::prelude::*;

use super::GameplaySet;
use super::core::{InGameEntity, Lifetime, MainCamera, Velocity, frand_range, layer};
use super::player::{Player, PlayerStatus};
use super::state::{GameState, PlayState};

const EXPLOSION_DURATION: f32 = 0.5;

#[derive(Component)]
struct ExplosionParticle;

#[derive(Component)]
pub(crate) struct CameraShake {
    timer: Timer,
    magnitude: f32,
}

#[derive(Message)]
pub struct ShakeEvent {
    pub magnitude: f32,
    pub duration: f32,
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
        app.add_message::<ShakeEvent>().add_systems(
            Update,
            (
                start_camera_shake,
                apply_camera_shake,
                fade_explosions,
                update_invincibility_visuals,
            )
                .chain()
                .in_set(GameplaySet::Fx)
                .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
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

fn start_camera_shake(
    mut commands: Commands,
    mut shake_reader: MessageReader<ShakeEvent>,
    mut query: Query<(
        Entity,
        &Transform,
        &mut MainCamera,
        Option<&mut CameraShake>,
    )>,
) {
    let Ok((entity, transform, mut camera, shake)) = query.single_mut() else {
        return;
    };

    let mut magnitude: f32 = 0.0;
    let mut duration: f32 = 0.0;
    for event in shake_reader.read() {
        magnitude = magnitude.max(event.magnitude);
        duration = duration.max(event.duration);
    }

    if magnitude <= 0.0 || duration <= 0.0 {
        return;
    }

    if let Some(mut shake) = shake {
        shake.magnitude = shake.magnitude.max(magnitude);
        let extended_duration = shake.timer.duration().as_secs_f32().max(duration);
        shake
            .timer
            .set_duration(std::time::Duration::from_secs_f32(extended_duration));
        shake.timer.reset();
    } else {
        camera.base = transform.translation;
        commands.entity(entity).insert(CameraShake {
            timer: Timer::from_seconds(duration, TimerMode::Once),
            magnitude,
        });
    }
}

fn apply_camera_shake(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &MainCamera, &mut CameraShake)>,
) {
    for (entity, mut transform, camera, mut shake) in &mut query {
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
}

pub fn reset_camera_shake(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &MainCamera, &CameraShake)>,
) {
    for (entity, mut transform, camera, _) in &mut query {
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

fn update_invincibility_visuals(
    time: Res<Time>,
    mut query: Query<(&mut PlayerStatus, &mut Sprite), With<Player>>,
) {
    for (mut status, mut sprite) in &mut query {
        let Some(timer) = status.invincible.as_mut() else {
            sprite.color = Color::WHITE;
            continue;
        };

        timer.tick(time.delta());

        let elapsed = timer.elapsed_secs();
        let visible = (elapsed * 10.0) as i32 % 2 == 0;
        sprite.color = if visible {
            Color::WHITE
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.3)
        };

        if timer.is_finished() {
            status.invincible = None;
            sprite.color = Color::WHITE;
        }
    }
}
