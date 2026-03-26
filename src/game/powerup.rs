use bevy::prelude::*;

use super::GameplaySet;
use super::core::{Collider, Health, InGameEntity, OffscreenDespawn, Velocity, layer};
use super::player::{PierceShot, Player, RapidFire, TripleShot};
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

    pub fn apply(self, commands: &mut Commands, player: Entity, health: &mut Health) {
        match self {
            Self::TripleShot => {
                commands
                    .entity(player)
                    .insert(TripleShot::new(POWERUP_TRIPLE_DURATION));
            }
            Self::RapidFire => {
                commands
                    .entity(player)
                    .insert(RapidFire::new(POWERUP_RAPID_DURATION));
            }
            Self::PierceShot => {
                commands
                    .entity(player)
                    .insert(PierceShot::new(POWERUP_PIERCE_DURATION));
            }
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
    mut player: Query<(Entity, &Transform, &Collider, &mut Health), With<Player>>,
) {
    let Ok((player_entity, player_transform, player_collider, mut health)) = player.single_mut()
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

        item.0.apply(&mut commands, player_entity, &mut health);
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::player::{PierceShot, RapidFire, TripleShot};
    use bevy::app::App;

    #[test]
    fn powerups_apply_expected_state_changes() {
        let mut app = App::new();
        let player = app.world_mut().spawn_empty().id();
        let mut health = Health::new(3);
        health.current = 2;

        PowerUpKind::TripleShot.apply(&mut app.world_mut().commands(), player, &mut health);
        PowerUpKind::RapidFire.apply(&mut app.world_mut().commands(), player, &mut health);
        PowerUpKind::PierceShot.apply(&mut app.world_mut().commands(), player, &mut health);
        PowerUpKind::Shield.apply(&mut app.world_mut().commands(), player, &mut health);
        app.update();

        assert!(app.world().entity(player).contains::<TripleShot>());
        assert!(app.world().entity(player).contains::<RapidFire>());
        assert!(app.world().entity(player).contains::<PierceShot>());
        assert_eq!(health.current, 3);
    }
}
