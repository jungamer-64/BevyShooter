use bevy::prelude::*;

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

#[derive(Event, Debug, Clone, Copy)]
pub struct DespawnRequestedEvent(pub Entity);

#[derive(Event, Debug, Clone, Copy)]
pub struct EnemyHitEvent(pub Entity);

#[derive(Event, Debug, Clone, Copy)]
pub struct EnemyDestroyedEvent {
    pub entity: Entity,
    pub position: Vec3,
    pub score: u32,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct PlayerDamagedEvent {
    pub player: Entity,
    pub defeated: bool,
    pub consumed: Entity,
}
