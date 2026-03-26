use bevy::prelude::*;
use bevy::window::WindowPlugin;

use crate::game::GamePlugin;
use crate::platform;

pub fn build() -> App {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(platform::primary_window()),
                ..default()
            })
            .set(platform::asset_plugin()),
    );

    platform::configure_app(&mut app);
    app.add_plugins(GamePlugin);
    app
}

pub fn run() {
    let mut app = build();
    app.run();
}
