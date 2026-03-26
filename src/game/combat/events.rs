use bevy::prelude::*;

use super::super::powerup::PowerUpKind;

#[derive(Message, Debug, Clone, Copy)]
pub struct BulletEnemyContact {
    pub bullet: Entity,
    pub enemy: Entity,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct PlayerEnemyContact {
    pub player: Entity,
    pub enemy: Entity,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct PlayerBulletContact {
    pub player: Entity,
    pub bullet: Entity,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct DespawnRequest(pub Entity);

#[derive(Message, Debug, Clone, Copy)]
pub struct EnemyHit(pub Entity);

#[derive(Message, Debug, Clone, Copy)]
pub struct EnemyDestroyed {
    pub entity: Entity,
    pub position: Vec3,
    pub score: u32,
    pub drop: Option<PowerUpKind>,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct PlayerDamaged {
    pub player: Entity,
    pub defeated: bool,
    pub consumed: Entity,
}
