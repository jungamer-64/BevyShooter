use bevy::prelude::*;

use super::GameplaySet;
use super::core::{Collider, Health, InGameEntity, OffscreenDespawn, Velocity, layer};
use super::player::{Player, PlayerWeapons};
use super::state::{GameState, PlayState};

const POWERUP_DROP_RATE: f32 = 0.3;
const POWERUP_SPEED: f32 = 100.0;
const POWERUP_TRIPLE_DURATION: f32 = 10.0;
const POWERUP_RAPID_DURATION: f32 = 8.0;
const POWERUP_PIERCE_DURATION: f32 = 12.0;
const POWERUP_SIZE: Vec2 = Vec2::new(20.0, 20.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerUpKind {
    TripleShot,
    RapidFire,
    Shield,
    PierceShot,
}

impl PowerUpKind {
    fn color(self) -> Color {
        match self {
            Self::TripleShot => Color::srgb(0.2, 0.6, 1.0),
            Self::RapidFire => Color::srgb(1.0, 1.0, 0.2),
            Self::PierceShot => Color::srgb(0.8, 0.2, 1.0),
            Self::Shield => Color::srgb(0.2, 1.0, 0.2),
        }
    }

    pub fn apply(self, weapons: &mut PlayerWeapons, health: &mut Health) {
        match self {
            Self::TripleShot => weapons.activate_triple_shot(POWERUP_TRIPLE_DURATION),
            Self::RapidFire => weapons.activate_rapid_fire(POWERUP_RAPID_DURATION),
            Self::PierceShot => weapons.activate_pierce_shot(POWERUP_PIERCE_DURATION),
            Self::Shield => health.heal(1),
        }
    }
}

#[derive(Component)]
pub struct PowerUpPickup(pub PowerUpKind);

#[derive(Bundle)]
struct PowerUpBundle {
    item: PowerUpPickup,
    sprite: Sprite,
    transform: Transform,
    velocity: Velocity,
    collider: Collider,
    offscreen: OffscreenDespawn,
    cleanup: InGameEntity,
}

impl PowerUpBundle {
    fn new(position: Vec3, kind: PowerUpKind) -> Self {
        Self {
            item: PowerUpPickup(kind),
            sprite: Sprite::from_color(kind.color(), POWERUP_SIZE),
            transform: Transform::from_xyz(position.x, position.y, layer::POWERUP),
            velocity: Velocity(Vec2::new(-POWERUP_SPEED, 0.0)),
            collider: Collider { size: POWERUP_SIZE },
            offscreen: OffscreenDespawn::horizontal(80.0),
            cleanup: InGameEntity,
        }
    }
}

pub struct PowerUpPlugin;

impl Plugin for PowerUpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            collect_powerups
                .in_set(GameplaySet::Detect)
                .run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
        );
    }
}

pub fn spawn_pickup(commands: &mut Commands, position: Vec3, kind: PowerUpKind) {
    commands.spawn(PowerUpBundle::new(position, kind));
}

pub fn roll_drop() -> Option<PowerUpKind> {
    if fastrand::f32() >= POWERUP_DROP_RATE {
        return None;
    }

    Some(match fastrand::u32(0..4) {
        0 => PowerUpKind::TripleShot,
        1 => PowerUpKind::RapidFire,
        2 => PowerUpKind::PierceShot,
        _ => PowerUpKind::Shield,
    })
}

fn collect_powerups(
    mut commands: Commands,
    powerups: Query<(Entity, &Transform, &Collider, &PowerUpPickup)>,
    mut player: Query<(&Transform, &Collider, &mut PlayerWeapons, &mut Health), With<Player>>,
) {
    let Ok((player_transform, player_collider, mut weapons, mut health)) = player.single_mut()
    else {
        return;
    };

    for (entity, transform, collider, item) in &powerups {
        if !player_collider.intersects(
            player_transform.translation,
            *collider,
            transform.translation,
        ) {
            continue;
        }

        item.0.apply(&mut weapons, &mut health);
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::player::PlayerWeapons;

    #[test]
    fn powerups_apply_expected_state_changes() {
        let mut weapons = PlayerWeapons::default();
        let mut health = Health::new(3);
        health.current = 2;

        PowerUpKind::TripleShot.apply(&mut weapons, &mut health);
        PowerUpKind::RapidFire.apply(&mut weapons, &mut health);
        PowerUpKind::PierceShot.apply(&mut weapons, &mut health);
        PowerUpKind::Shield.apply(&mut weapons, &mut health);

        assert!(weapons.remaining_triple_shot().is_some());
        assert!(weapons.remaining_rapid_fire().is_some());
        assert!(weapons.remaining_pierce_shot().is_some());
        assert_eq!(health.current, 3);
    }
}
