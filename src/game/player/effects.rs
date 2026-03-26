use bevy::prelude::*;

use super::components::{
    Invincible, PierceShot, Player, PlayerEffectSnapshot, PlayerWeapons, RapidFire,
    TimedEffectComponent, TripleShot,
};

pub fn tick_player_cooldown(time: Res<Time>, mut weapons: Single<&mut PlayerWeapons, With<Player>>) {
    weapons.tick_cooldown(time.delta());
}

pub fn tick_temporary_effects(
    mut commands: Commands,
    time: Res<Time>,
    player: Single<
        (
            Entity,
            Option<&mut TripleShot>,
            Option<&mut RapidFire>,
            Option<&mut PierceShot>,
            Option<&mut Invincible>,
        ),
        With<Player>,
    >,
) {
    let (entity, triple_shot, rapid_fire, pierce_shot, invincible) = player.into_inner();

    tick_effect::<TripleShot>(
        &mut commands,
        entity,
        triple_shot,
        time.delta(),
        "Triple Shot",
    );
    tick_effect::<RapidFire>(
        &mut commands,
        entity,
        rapid_fire,
        time.delta(),
        "Rapid Fire",
    );
    tick_effect::<PierceShot>(
        &mut commands,
        entity,
        pierce_shot,
        time.delta(),
        "Pierce Shot",
    );
    tick_effect::<Invincible>(
        &mut commands,
        entity,
        invincible,
        time.delta(),
        "Invincibility",
    );
}

pub fn update_invincibility_visuals(
    player: Single<
        (&mut Sprite, Option<&TripleShot>, Option<&RapidFire>, Option<&PierceShot>, Option<&Invincible>),
        With<Player>,
    >,
) {
    let (mut sprite, triple_shot, rapid_fire, pierce_shot, invincible) = player.into_inner();
    let effects =
        PlayerEffectSnapshot::from_components(triple_shot, rapid_fire, pierce_shot, invincible);

    if !effects.is_invincible() {
        sprite.color = Color::WHITE;
        return;
    }

    sprite.color = if effects.invincible_visible() {
        Color::WHITE
    } else {
        Color::srgba(1.0, 1.0, 1.0, 0.3)
    };
}

fn tick_effect<T: Component + TimedEffectComponent>(
    commands: &mut Commands,
    entity: Entity,
    effect: Option<Mut<T>>,
    delta: std::time::Duration,
    _label: &str,
) {
    let Some(mut effect) = effect else {
        return;
    };

    effect.tick(delta);
    if effect.is_finished() {
        crate::dlog!("{_label} expired!");
        commands.entity(entity).remove::<T>();
    }
}
