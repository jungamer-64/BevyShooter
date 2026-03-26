use bevy::prelude::*;

use super::super::combat::EnemyHitEvent;
use super::super::core::Health;
use super::components::{Enemy, EnemyHitFlash};

pub fn on_enemy_hit(
    event: On<EnemyHitEvent>,
    mut commands: Commands,
    enemies: Query<(), With<Enemy>>,
) {
    if enemies.get(event.0).is_ok() {
        commands.entity(event.0).insert(EnemyHitFlash::new());
    }
}

pub fn update_enemy_visuals(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, Ref<Health>, Option<&mut EnemyHitFlash>, &mut Sprite), With<Enemy>>,
) {
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
