/// Define the controls for the camera and handle user input.
use super::camera::Camera;

use winit::{
    event::*,
    keyboard::{KeyCode, PhysicalKey},
};
pub struct CameraController {
    speed: f32,
    sensitivity: f32,
    // Movement controls
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,     // Added for Space
    is_down_pressed: bool,   // Added for Shift
    // Rotation controls (arrow keys)
    is_looking_left: bool,
    is_looking_right: bool,
    is_looking_up: bool,
    is_looking_down: bool,
    // Camera help controls
    is_h_pressed: bool,
    is_being_helped: bool,
    is_h_just_pressed: bool,
    // Saved camera fields
    eyecpy: cgmath::Point3<f32>,
    targetcpy: cgmath::Point3<f32>,
    // Camera rotation state
    yaw: f32,   // Left/right rotation
    pitch: f32, // Up/down rotation
}

impl CameraController {
    /// Create new camera controller
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            sensitivity: 0.1,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
            is_looking_left: false,
            is_looking_right: false,
            is_looking_up: false,
            is_looking_down: false,
            is_h_pressed: false,
            is_being_helped: true,
            is_h_just_pressed: false,
            yaw: -90.0,   // Start looking along -Z
            pitch: 0.0,
            eyecpy: (0.0, 1.0, 2.0).into(),
            targetcpy: (0.0, 0.0, 0.0).into(),
        }
    }

    /// Process window events to move the camera
    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state,
                    physical_key: PhysicalKey::Code(keycode),
                    repeat: false,
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    // WASD controls
                    KeyCode::KeyW => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    // Up/Down controls
                    KeyCode::Space => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    KeyCode::ShiftLeft => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    // Arrow key controls
                    KeyCode::ArrowLeft => {
                        self.is_looking_left = is_pressed;
                        true
                    }
                    KeyCode::ArrowRight => {
                        self.is_looking_right = is_pressed;
                        true
                    }
                    KeyCode::ArrowUp => {
                        self.is_looking_up = is_pressed;
                        true
                    }
                    KeyCode::ArrowDown => {
                        self.is_looking_down = is_pressed;
                        true
                    }
                    // help menu toggle
                    KeyCode::KeyH => {
                        self.is_h_pressed = is_pressed;
                        if is_pressed && !self.is_being_helped {
                            self.is_being_helped = true;
                        }
                        else if is_pressed && self.is_being_helped {
                            self.is_h_just_pressed = true;
                            self.is_being_helped = false;
                        }
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Modified to always process mouse movement without button check
    pub fn process_mouse(&mut self, dx: f64, dy: f64) {
        // if not in the help menu
        if !self.is_being_helped {
            // Always process mouse movement
            self.yaw += dx as f32 * self.sensitivity;
            self.pitch -= dy as f32 * self.sensitivity;

            // Allow full 360-degree horizontal rotation
            if self.yaw > 360.0 {
                self.yaw -= 360.0;
            } else if self.yaw < -360.0 {
                self.yaw += 360.0;
            }
            
            // Constrain pitch to prevent camera flipping
            self.pitch = self.pitch.clamp(-89.0, 89.0);
        }
    }

    /// Modified to always process mouse wheel without button check
    pub fn process_mouse_wheel(&mut self, scroll:f32, camera: &mut Camera) {
        if !self.is_being_helped {
            use cgmath::InnerSpace;

            let (yaw_rad, pitch_rad) = (
                self.yaw.to_radians(),
                self.pitch.to_radians(),
            );

            let front = cgmath::Vector3::new(
                yaw_rad.cos() * pitch_rad.cos(),
                pitch_rad.sin(),
                yaw_rad.sin() * pitch_rad.cos(),
            ).normalize();

            camera.eye += scroll * front * self.speed * 50.0;
        }
    }

    /// Update the camera based of what is pressed
    pub fn update_camera(&mut self, camera: &mut Camera) {
        use cgmath::InnerSpace;
            self.go_to_help(camera);

        if !self.is_being_helped {
            // Handle rotation from arrow keys
            let rotation_speed = 0.5;
            if self.is_looking_left {
                self.yaw -= rotation_speed;
            }
            if self.is_looking_right {
                self.yaw += rotation_speed;
            }
            if self.is_looking_up {
                self.pitch += rotation_speed;
            }
            if self.is_looking_down {
                self.pitch -= rotation_speed;
            }

            // Constrain pitch to prevent camera flipping
            self.pitch = self.pitch.clamp(-89.0, 89.0);

            // Calculate new front direction
            let (yaw_rad, pitch_rad) = (
                self.yaw.to_radians(),
                self.pitch.to_radians(),
            );
            
            let front = cgmath::Vector3::new(
                yaw_rad.cos() * pitch_rad.cos(),
                pitch_rad.sin(),
                yaw_rad.sin() * pitch_rad.cos(),
            ).normalize();

            // Calculate right vector
            let right = front.cross(camera.up).normalize();

            // Update movement based on where we're looking
            if self.is_forward_pressed {
                camera.eye += front * self.speed;
            }
            if self.is_backward_pressed {
                camera.eye -= front * self.speed;
            }
            if self.is_right_pressed {
                camera.eye += right * self.speed;
            }
            if self.is_left_pressed {
                camera.eye -= right * self.speed;
            }
            
            // Handle up/down movement
            if self.is_up_pressed {
                camera.eye.y += self.speed;
            }
            if self.is_down_pressed {
                camera.eye.y -= self.speed;
            }
        
            // Update where we're looking
            camera.target = camera.eye + front;

        }
    }

    // sets the camera's position and direction to face the help menu
    pub fn go_to_help(&mut self, camera: &mut Camera) {
        // set camera direction to help cube location if not already in menu
        if self.is_being_helped {
            camera.eye = (0.0, 0.0, 2.0).into();
            camera.target = (0.0, 0.0, 0.0).into();
        }
        // set camera direction to previous state if already in menu
        if !self.is_being_helped {
            if self.is_h_just_pressed {
                camera.eye = self.eyecpy;
                camera.target = self.targetcpy;
                self.is_h_just_pressed = false;
            }
            self.eyecpy = camera.eye.clone();
            self.targetcpy = camera.target.clone();
        }
    }
}
