use bevy::prelude::*;
#[cfg(all(feature = "dev-logging", debug_assertions))]
use std::time::Duration;

#[derive(Resource, Default)]
pub struct GameAssets {
    spaceship: Handle<Image>,
    bullet: Handle<Image>,
    asteroid: Handle<Image>,
}

impl GameAssets {
    pub fn spaceship(&self) -> Handle<Image> {
        self.spaceship.clone()
    }

    pub fn bullet(&self) -> Handle<Image> {
        self.bullet.clone()
    }

    pub fn asteroid(&self) -> Handle<Image> {
        self.asteroid.clone()
    }
}

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .add_systems(Startup, load_assets)
            .add_systems(
                PreUpdate,
                debug_asset_loading.run_if(|| cfg!(all(feature = "dev-logging", debug_assertions))),
            );
    }
}

fn load_assets(mut game_assets: ResMut<GameAssets>, asset_server: Res<AssetServer>) {
    game_assets.spaceship = asset_server.load("spaceship.webp");
    game_assets.bullet = asset_server.load("bullet.webp");
    game_assets.asteroid = asset_server.load("asteroid.webp");
}

#[cfg(all(feature = "dev-logging", debug_assertions))]
fn debug_asset_loading(
    asset_server: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    mut diag_timer: Local<Timer>,
    time: Res<Time>,
) {
    if diag_timer.duration() == Duration::ZERO {
        *diag_timer = Timer::from_seconds(1.0, TimerMode::Repeating);
    }

    diag_timer.tick(time.delta());
    if !diag_timer.just_finished() {
        return;
    }

    let log_status = |name: &str, handle: &Handle<Image>| {
        if let Some(path) = handle.path() {
            crate::dlog!(
                "Asset {}: {:?} ({:?})",
                name,
                asset_server.load_state(handle),
                path
            );
        } else {
            crate::dlog!(
                "Asset {}: {:?} (No path)",
                name,
                asset_server.load_state(handle)
            );
        }
    };

    log_status("Spaceship", &game_assets.spaceship);
    log_status("Bullet", &game_assets.bullet);
    log_status("Asteroid", &game_assets.asteroid);
}

#[cfg(not(all(feature = "dev-logging", debug_assertions)))]
fn debug_asset_loading(
    _asset_server: Res<AssetServer>,
    _game_assets: Res<GameAssets>,
    _diag_timer: Local<Timer>,
    _time: Res<Time>,
) {
}
