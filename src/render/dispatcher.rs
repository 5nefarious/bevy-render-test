use super::renderer::*;
use bevy::{
    prelude::*,
    window::WindowCreated,
};
use futures::executor::block_on;

pub struct Dispatcher {
    instance: wgpu::Instance,
}

impl Dispatcher {
    pub fn new() -> Self {
        Dispatcher {
            instance: wgpu::Instance::new(wgpu::Backends::all()),
        }
    }

    pub async fn new_renderer(
        &self,
        window: &Window,
    ) -> Renderer {
        let window_id = window.id();
        eprintln!("Spawning new renderer for window {window_id}");
        Renderer::new(&self.instance, window).await
    }
}

pub fn dispatcher_system(
    mut window_created_events: EventReader<WindowCreated>,
    windows: NonSend<Windows>,
    dispatcher: Res<Dispatcher>,
    mut commands: Commands,
) {
    for event in window_created_events.iter() {
        let window = windows
            .get(event.id)
            .expect("Received event from nonexistent window");
        let renderer = block_on(
            dispatcher.new_renderer(window)
        );
        commands.spawn().insert(renderer);
    }
}