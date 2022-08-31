use super::{
    dispatcher::*,
    renderer::render_system,
};
use bevy::prelude::*;

#[derive(Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Dispatcher::new())
            .add_system(dispatcher_system.exclusive_system())
            .add_system(render_system);
    }
}