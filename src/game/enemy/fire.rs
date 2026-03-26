use bevy::prelude::*;

use super::super::player::Player;
use super::components::{ENEMY_BULLET_SPEED, Enemy, EnemyBulletBundle, EnemyFireTimer};

pub fn enemy_fire_system(
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
