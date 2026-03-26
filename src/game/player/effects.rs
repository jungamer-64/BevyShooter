use bevy::prelude::*;

use super::components::{
    Invincible, PierceShot, Player, PlayerWeapons, RapidFire, TimedEffectComponent, TripleShot,
};

pub fn tick_player_cooldown(time: Res<Time>, mut query: Query<&mut PlayerWeapons, With<Player>>) {
    let Ok(mut weapons) = query.single_mut() else {
        return;
    };

    weapons.tick_cooldown(time.delta());
}

pub fn tick_temporary_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<
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
    let Ok((entity, triple_shot, rapid_fire, pierce_shot, invincible)) = query.single_mut() else {
        return;
    };

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
