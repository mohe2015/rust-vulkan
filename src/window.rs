// Copyright (c) 2021 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::{renderer::PoritzCraftRenderer, utils::state_is_pressed};

use winit::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub struct PoritzCraftWindow {}

impl PoritzCraftWindow {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(&self) {
        let event_loop = EventLoop::new();

        let mut renderer = PoritzCraftRenderer::new(&event_loop);

        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                renderer.main_pipeline.recreate_swapchain = true;
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if let Some(key_code) = input.virtual_keycode {
                    match key_code {
                        VirtualKeyCode::LControl => {
                            renderer.main_pipeline.control = state_is_pressed(input.state)
                        }
                        VirtualKeyCode::W => {
                            renderer.main_pipeline.pan_up = state_is_pressed(input.state)
                        }
                        VirtualKeyCode::A => {
                            renderer.main_pipeline.pan_left = state_is_pressed(input.state)
                        }
                        VirtualKeyCode::S => {
                            renderer.main_pipeline.pan_down = state_is_pressed(input.state)
                        }
                        VirtualKeyCode::D => {
                            renderer.main_pipeline.pan_right = state_is_pressed(input.state)
                        }
                        _ => (),
                    }
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: _,
                        button: _,
                        ..
                    },
                ..
            } => {}
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position: _, .. },
                ..
            } => {}
            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta: _, .. },
                ..
            } => {}
            Event::RedrawEventsCleared => {
                renderer.main_pipeline.render();
            }
            _ => (),
        });
    }
}
