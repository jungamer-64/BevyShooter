use bevy::prelude::*;

use super::super::assets::GameAssets;
use super::super::combat::{HitList, Pierce};
use super::components::{
    BULLET_SPEED, BulletBundle, PIERCE_SHOT_CHARGES, PierceShot, Player, PlayerIntent,
    PlayerWeapons, RapidFire, SINGLE_SHOT_ANGLES, SPREAD_SHOT_ANGLES, TripleShot,
};

pub fn player_shoot(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut query: Query<
        (
            &Transform,
            &PlayerIntent,
            &mut PlayerWeapons,
            Option<&TripleShot>,
            Option<&RapidFire>,
            Option<&PierceShot>,
        ),
        With<Player>,
    >,
) {
    let Ok((player_transform, intent, mut weapons, triple_shot, rapid_fire, pierce_shot)) =
        query.single_mut()
    else {
        return;
    };

    if !intent.firing || !weapons.ready_to_fire() {
        return;
    }

    let fire_angles = if triple_shot.is_some() {
        &SPREAD_SHOT_ANGLES[..]
    } else {
        &SINGLE_SHOT_ANGLES[..]
    };

    for &angle in fire_angles {
        let velocity = Vec2::new(angle.cos(), angle.sin()) * BULLET_SPEED;
        let mut bullet = commands.spawn(BulletBundle::new(
            &game_assets,
            player_transform.translation,
            velocity,
        ));

        if pierce_shot.is_some() {
            bullet.insert((Pierce(PIERCE_SHOT_CHARGES), HitList::default()));
        }
    }

    weapons.reset_fire_cooldown(rapid_fire.is_some());
}
