mod render;

use bevy::{
    prelude::*,
    core::CorePlugin,
    diagnostic::DiagnosticsPlugin,
    input::InputPlugin,
    log::LogPlugin,
    window::WindowPlugin,
    winit::WinitPlugin,
};
use render::RenderPlugin;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Render Test".to_string(),
            width: 640.0,
            height: 480.0,
            ..Default::default()
        })
        .add_plugin(LogPlugin::default())
        .add_plugin(CorePlugin::default())
        .add_plugin(DiagnosticsPlugin::default())
        .add_plugin(InputPlugin::default())
        .add_plugin(WindowPlugin::default())
        .add_plugin(WinitPlugin::default())
        .add_plugin(RenderPlugin::default())
        .run();
}