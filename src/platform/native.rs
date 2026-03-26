use bevy::app::App;
use bevy::asset::AssetPlugin;
use bevy::window::Window;

use super::base_window;

pub fn primary_window() -> Window {
    base_window()
}

pub fn asset_plugin() -> AssetPlugin {
    AssetPlugin::default()
}

pub fn configure_app(_app: &mut App) {}
