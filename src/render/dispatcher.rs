use super::renderer::*;
use bevy::{
    prelude::*,
    window::WindowCreated,
    winit::WinitWindows,
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
        winit_window: &winit::window::Window
    ) -> Renderer {
        Renderer::new(&self.instance, window, winit_window).await
    }
}

pub fn dispatcher_system(
    mut window_created_events: EventReader<WindowCreated>,
    windows: Res<Windows>,
    winit_windows: NonSend<WinitWindows>,
    dispatcher: Res<Dispatcher>,
    mut commands: Commands,
) {
    for event in window_created_events.iter() {
        let window = windows
            .get(event.id)
            .expect("Received event from nonexistent window");
        match winit_windows
            .get_window(event.id)
        {
            Some(winit_window) => {
                let renderer = block_on(
                    dispatcher.new_renderer(window, winit_window)
                );
                commands.spawn().insert(renderer);
            },
            None => {},
        }
    }
}