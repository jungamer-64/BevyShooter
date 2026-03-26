use bevy::prelude::*;
use std::time::Duration;

use super::super::assets::GameAssets;
use super::super::core::{GameBounds, frand_range};
use super::components::{
    ChasePlayer, Difficulty, EnemyBundle, EnemyType, SpawnState, ZigzagMotion,
};

const ENEMY_FIRE_INTERVAL: f32 = 2.0;
const SPAWN_INTERVAL_SECONDS: f32 = 1.0;
const DIFFICULTY_INTERVAL: f32 = 15.0;

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
    let mut roll = fastrand::u32(
        0..EnemyType::Normal.spawn_weight(level)
            + EnemyType::Zigzag.spawn_weight(level)
            + EnemyType::Chaser.spawn_weight(level),
    );

    for enemy_type in [EnemyType::Normal, EnemyType::Zigzag, EnemyType::Chaser] {
        let weight = enemy_type.spawn_weight(level);
        if roll < weight {
            return enemy_type;
        }
        roll -= weight;
    }

    EnemyType::Normal
}

pub(super) fn retime_spawn_state(spawn: &mut SpawnState, new_interval: f32) {
    let old_duration = spawn.timer.duration().as_secs_f32().max(0.0001);
    let fraction = (spawn.timer.elapsed_secs() / old_duration).clamp(0.0, 1.0);

    spawn.current_interval = new_interval;
    spawn
        .timer
        .set_duration(Duration::from_secs_f32(new_interval));
    spawn
        .timer
        .set_elapsed(Duration::from_secs_f32(fraction * new_interval));
}

pub fn reset_enemy_progress(mut spawn: ResMut<SpawnState>, mut difficulty: ResMut<Difficulty>) {
    *spawn = SpawnState::default();
    *difficulty = Difficulty::default();
}

pub fn update_difficulty(time: Res<Time>, mut difficulty: ResMut<Difficulty>) {
    difficulty.elapsed_time += time.delta_secs();
    let next_level = (difficulty.elapsed_time / DIFFICULTY_INTERVAL) as u32;

    if next_level > difficulty.level {
        difficulty.level = next_level;
        crate::dlog!("Difficulty increased to level {}", difficulty.level);
    }
}

pub fn sync_spawn_interval(mut spawn: ResMut<SpawnState>, difficulty: Res<Difficulty>) {
    let new_interval = spawn_interval_for_level(difficulty.level);
    if (new_interval - spawn.current_interval).abs() <= f32::EPSILON {
        return;
    }

    retime_spawn_state(&mut spawn, new_interval);
    crate::dlog!("Spawn interval changed to {:.2}s", new_interval);
}

pub fn enemy_spawner(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn: ResMut<SpawnState>,
    bounds: Res<GameBounds>,
    game_assets: Res<GameAssets>,
    difficulty: Res<Difficulty>,
) {
    spawn.timer.tick(time.delta());
    let spawn_count = spawn.timer.times_finished_this_tick().min(3);
    let speed_multiplier = speed_multiplier_for_level(difficulty.level);
    let fire_interval = enemy_fire_interval_for_level(difficulty.level);

    for _ in 0..spawn_count {
        let enemy_type = random_enemy_type(difficulty.level);
        let y = frand_range(bounds.spawn_y_range());
        let mut enemy = commands.spawn(EnemyBundle::new(
            enemy_type,
            &game_assets,
            &bounds,
            y,
            speed_multiplier,
            fire_interval,
        ));

        match enemy_type {
            EnemyType::Normal => {}
            EnemyType::Zigzag => {
                enemy.insert(ZigzagMotion {
                    phase: fastrand::f32() * std::f32::consts::TAU,
                });
            }
            EnemyType::Chaser => {
                enemy.insert(ChasePlayer);
            }
        }
    }
}
