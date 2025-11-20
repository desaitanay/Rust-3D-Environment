/// Handle grabbing the mouse when the window is focussed so that it doen't move around
use winit::{dpi::{PhysicalPosition, PhysicalSize}, event::{ElementState, KeyEvent, MouseButton, WindowEvent}, keyboard::{KeyCode, PhysicalKey}, window::Window};

/// Tool to keep mouse locked on screen when using
pub struct MouseGrabber {
    pub mouse_locked: bool,
}

impl MouseGrabber {
    /// Call this on every WindowEvent::CursorMoved to update the mouse to be in the center
    pub fn process_mouse(&mut self, window: &Window, size: &PhysicalSize<u32>) {
        if self.mouse_locked {
			window.set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2)).unwrap();
        }
    }

    /// Call this for button presses so we can lock and unlock the mouse
    pub fn process_events(&mut self, event: &WindowEvent, window: &Window) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state,
                    physical_key: PhysicalKey::Code(keycode),
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    // Escape unlocks the mouse
                    KeyCode::Escape => {
                        if is_pressed && self.mouse_locked {
                            self.mouse_locked = false;
                            window.set_cursor_visible(true);
                            return true
                        }
                        false
                    }
                    _ => false,
                }
            }
            WindowEvent::MouseInput {
                state,
                button,
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        if is_pressed && !self.mouse_locked {
                            self.mouse_locked = true;
                            window.set_cursor_visible(false);
                            return true
                        }
                        false
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}