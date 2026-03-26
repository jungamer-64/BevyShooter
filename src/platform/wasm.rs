use bevy::app::App;
use bevy::asset::{AssetMetaCheck, AssetPlugin};
use bevy::window::Window;

use super::base_window;

pub fn primary_window() -> Window {
    let mut window = base_window();
    window.fit_canvas_to_parent = true;
    window.prevent_default_event_handling = true;
    window
}

pub fn asset_plugin() -> AssetPlugin {
    AssetPlugin {
        meta_check: AssetMetaCheck::Never,
        ..Default::default()
    }
}

pub fn configure_app(_app: &mut App) {}
