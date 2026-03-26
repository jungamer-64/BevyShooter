use bevy::prelude::*;

use super::GameplaySet;
use super::combat::{DespawnRequestedEvent, EnemyDestroyedEvent};
use super::core::{Collider, Health, InGameEntity, OffscreenDespawn, Velocity, layer};
use super::player::{PierceShot, Player, RapidFire, TripleShot};

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
}

#[derive(Component)]
pub struct PowerUpPickup(pub PowerUpKind);

#[derive(Event, Debug, Clone, Copy)]
pub struct PowerUpCollectedEvent {
    pub pickup: Entity,
    pub player: Entity,
    pub kind: PowerUpKind,
}

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
        app.add_observer(on_powerup_collected)
            .add_observer(spawn_drop_on_enemy_destroyed)
            .add_systems(
            Update,
            collect_powerups.in_set(GameplaySet::Detect),
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
    player: Single<(Entity, &Transform, &Collider), With<Player>>,
) {
    let (player_entity, player_transform, player_collider) = player.into_inner();

    for (entity, transform, collider, item) in &powerups {
        if !player_collider.intersects(
            player_transform.translation,
            *collider,
            transform.translation,
        ) {
            continue;
        }

        commands.trigger(PowerUpCollectedEvent {
            pickup: entity,
            player: player_entity,
            kind: item.0,
        });
    }
}

fn on_powerup_collected(
    event: On<PowerUpCollectedEvent>,
    mut commands: Commands,
    mut players: Query<&mut Health, With<Player>>,
) {
    let Ok(mut health) = players.get_mut(event.player) else {
        return;
    };

    match event.kind {
        PowerUpKind::TripleShot => {
            commands
                .entity(event.player)
                .insert(TripleShot::new(POWERUP_TRIPLE_DURATION));
        }
        PowerUpKind::RapidFire => {
            commands
                .entity(event.player)
                .insert(RapidFire::new(POWERUP_RAPID_DURATION));
        }
        PowerUpKind::PierceShot => {
            commands
                .entity(event.player)
                .insert(PierceShot::new(POWERUP_PIERCE_DURATION));
        }
        PowerUpKind::Shield => health.heal(1),
    }

    commands.trigger(DespawnRequestedEvent(event.pickup));
}

fn spawn_drop_on_enemy_destroyed(event: On<EnemyDestroyedEvent>, mut commands: Commands) {
    if let Some(kind) = roll_drop() {
        spawn_pickup(&mut commands, event.position, kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::core::GameCorePlugin;
    use crate::game::player::{PierceShot, RapidFire, TripleShot};
    use bevy::app::App;

    #[test]
    fn powerups_apply_expected_state_changes() {
        let mut app = App::new();
        app.add_plugins(PowerUpPlugin);
        let player = app.world_mut().spawn((Player, Health::new(3))).id();
        let shield = app
            .world_mut()
            .spawn((PowerUpPickup(PowerUpKind::Shield),))
            .id();
        {
            let mut health = app
                .world_mut()
                .get_mut::<Health>(player)
                .expect("player health");
            health.current = 2;
        }

        app.world_mut().trigger(PowerUpCollectedEvent {
            pickup: shield,
            player,
            kind: PowerUpKind::TripleShot,
        });
        app.world_mut().trigger(PowerUpCollectedEvent {
            pickup: shield,
            player,
            kind: PowerUpKind::RapidFire,
        });
        app.world_mut().trigger(PowerUpCollectedEvent {
            pickup: shield,
            player,
            kind: PowerUpKind::PierceShot,
        });
        app.world_mut().trigger(PowerUpCollectedEvent {
            pickup: shield,
            player,
            kind: PowerUpKind::Shield,
        });
        app.world_mut().flush();

        assert!(app.world().entity(player).contains::<TripleShot>());
        assert!(app.world().entity(player).contains::<RapidFire>());
        assert!(app.world().entity(player).contains::<PierceShot>());
        assert_eq!(
            app.world()
                .entity(player)
                .get::<Health>()
                .expect("player health")
                .current,
            3
        );
    }

    #[test]
    fn collecting_powerup_despawns_pickup_and_applies_effect() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins((GameCorePlugin, PowerUpPlugin));

        let player = app.world_mut().spawn((
            Player,
            Transform::default(),
            Collider {
                size: Vec2::splat(20.0),
            },
            Health::new(3),
        ))
        .id();

        let pickup = app.world_mut().spawn((
            PowerUpPickup(PowerUpKind::Shield),
            Transform::default(),
            Collider {
                size: Vec2::splat(20.0),
            },
        ))
        .id();

        {
            let mut health = app
                .world_mut()
                .get_mut::<Health>(player)
                .expect("player health");
            health.current = 2;
        }

        app.update();

        assert!(!app.world().entities().contains(pickup));
        assert_eq!(
            app.world()
                .entity(player)
                .get::<Health>()
                .expect("player health")
                .current,
            3
        );
    }
}
