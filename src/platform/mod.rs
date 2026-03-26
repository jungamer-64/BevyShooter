use bevy::app::App;
use bevy::asset::AssetPlugin;
use bevy::window::{Window, WindowResolution};

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
use native as implementation;
#[cfg(target_arch = "wasm32")]
use wasm as implementation;

const WINDOW_TITLE: &str = "Bevy Shooter";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

fn base_window() -> Window {
    Window {
        title: WINDOW_TITLE.into(),
        resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
        ..Default::default()
    }
}

pub fn primary_window() -> Window {
    implementation::primary_window()
}

pub fn asset_plugin() -> AssetPlugin {
    implementation::asset_plugin()
}

pub fn configure_app(app: &mut App) {
    implementation::configure_app(app);
}
