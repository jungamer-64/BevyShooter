use bevy::prelude::*;

use super::super::assets::GameAssets;
use super::super::combat::{HitList, Pierce};
use super::components::{
    BULLET_SPEED, BulletBundle, PIERCE_SHOT_CHARGES, PierceShot, Player, PlayerIntent,
    PlayerEffectSnapshot, PlayerWeapons, RapidFire, SINGLE_SHOT_ANGLES, SPREAD_SHOT_ANGLES,
    TripleShot,
};

pub fn player_shoot(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    player: Single<
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
    let (player_transform, intent, mut weapons, triple_shot, rapid_fire, pierce_shot) =
        player.into_inner();
    let effects =
        PlayerEffectSnapshot::from_components(triple_shot, rapid_fire, pierce_shot, None);

    if !intent.firing || !weapons.ready_to_fire() {
        return;
    }

    let fire_angles = if effects.has_triple_shot() {
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

        if effects.has_pierce_shot() {
            bullet.insert((Pierce(PIERCE_SHOT_CHARGES), HitList::default()));
        }
    }

    weapons.reset_fire_cooldown(effects.has_rapid_fire());
}
