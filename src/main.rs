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
    App::build()
        .add_plugin(LogPlugin::default())
        .add_plugin(CorePlugin::default())
        .add_plugin(DiagnosticsPlugin::default())
        .add_plugin(InputPlugin::default())
        .add_plugin(WindowPlugin::default())
        .add_plugin(WinitPlugin::default())
        .add_plugin(RenderPlugin::default())
        .run();
}