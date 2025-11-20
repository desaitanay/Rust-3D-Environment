/// Define available library functions and setup our window
mod state;

use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

pub async fn run() {
    // Window setup...

    env_logger::init();

    // establish the event loop
    let event_loop = EventLoop::new().unwrap();

    // create the window
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // set up the state of the window
    let mut state = state::State::new(&window).await;
    
    // here we set what the event loop actually does
    let _ = event_loop.run(move |event, control_flow| {
        match event {
            // Handle mouse movement separate from window events
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                state.process_mouse_movement(delta.0, delta.1);
            },

            // Handle events in the window
            Event::WindowEvent {
                ref event,
                window_id,
            // Make sure the event is in the window and check if the event should be handled by the state instead
            } if window_id == state.window().id() => if !state.input(event) {
                match event {
                    // If window close requested, or key pressed then close window
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                repeat: false,
                                ..
                            },
                        ..
                    } => control_flow.exit(),

                    // If someone tries to resize the window, allow it
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    },

                    WindowEvent::MouseWheel { 
                        delta,
                        ..
                    } => {
                        state.process_mouse_wheel(delta);
                    },
                    
                    // Event to redraw the screen
                    WindowEvent::RedrawRequested => {
                        // This tells winit that we want another frame after this one
                        state.window().request_redraw();

                        // if !surface_configured {
                        //     return;
                        // }
                        
                        // update and render the screen
                        state.update();
                        match state.render() {
                            Ok(_) => {}
                            // Reconfigure the surface if it's lost or outdated
                            Err(
                                wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                            ) => state.resize(state.size),
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                log::error!("OutOfMemory");
                                control_flow.exit();
                            }

                            // This happens when the a frame takes too long to present
                            Err(wgpu::SurfaceError::Timeout) => {
                                log::warn!("Surface timeout")
                            }
                        }
                    }
                    // Catch all
                    _ => {}
                }
            }
            _ => {}
        }
    });
}