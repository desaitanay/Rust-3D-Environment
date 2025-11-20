/// Represents the overall world with all its models.
use std::{future::Future, rc::Rc};
use futures::{future::join_all, stream::FuturesUnordered, StreamExt};

use model::{DrawModel, Model};
use resources::{load_model, load_string};
use wgpu::BindGroupLayout;

use winit::{event::{ElementState, KeyEvent, WindowEvent}, keyboard::{KeyCode, PhysicalKey}};
use cgmath::prelude::*;

pub mod instance;
pub mod model;
pub mod resources;
pub mod texture;

pub struct World {
    // model vector
    pub models: Vec<Model>, 
    // model's cube's features
    is_increase_pressed: bool,
    is_decrease_pressed: bool,
    is_spin: bool,
    is_spin_pressed: bool,
    is_color_change: bool,
    is_color_change_pressed: bool,
    cur_angle: f32,
    cur_scale: f32,
    is_resize: bool,
    is_resize_pressed: bool,
    is_upscalling: bool,
    num_instances: u32,
    // initialization flag
    initialized: bool,
    // world help controls
    is_help_pressed: bool,
    is_being_helped: bool,
    is_help_just_pressed: bool
}

impl World {
    /// Create a new world by loading all possible models and textures
    pub async fn new(device: &Rc<wgpu::Device>, queue: &wgpu::Queue, texture_bind_group_layout: &BindGroupLayout) -> World {
        // we'll use a cube for now

        // load all the models specified in "resources.txt"
        let models = load_string(&"resources.txt")
            .await
            .unwrap()
            .split("\n")
            .map(|file_name| {
                load_model(file_name.trim_right(), device.clone(), queue, texture_bind_group_layout)
            }).collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        Self {
            models,
            is_decrease_pressed: false,
            is_increase_pressed: false,
            is_spin: false,
            is_spin_pressed: false,
            is_color_change: false,
            is_color_change_pressed: false,        
            cur_angle: 0.0,
            cur_scale: 1.0,
            is_resize: false,
            is_resize_pressed: false,
            is_upscalling: false,        
            num_instances: 5,
            initialized: true,
            is_help_pressed: false,
            is_being_helped: true,
            is_help_just_pressed: false
        }
    }

    /// handle window events
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
                    // increase number of cubes
                    KeyCode::KeyJ => {
                        self.is_increase_pressed = is_pressed;
                        true
                    }
                    // decrease number of cubes
                    KeyCode::KeyK => {
                        self.is_decrease_pressed = is_pressed;
                        true
                    }
                    // toggle is_spin if the key is pressed or released 
                    KeyCode::Digit1 => {
                        if is_pressed && !self.is_spin_pressed {
                            self.is_spin = !self.is_spin;
                            self.is_spin_pressed = true;
                        } else if !is_pressed && self.is_spin_pressed{
                            self.is_spin_pressed = false;
                        }
                        true
                    }
                    // toggle is_color_change if the key is pressed or released 
                    KeyCode::Digit2 => {
                        if is_pressed {
                            self.is_color_change = true;
                        } else {
                            self.is_color_change = false;
                        }
                        true
                    }
                    // toggle resize if the key is pressed or released
                    KeyCode::Digit3 => {
                        if is_pressed && !self.is_resize_pressed {
                            self.is_resize = !self.is_resize;
                            self.is_resize_pressed = true;
                        } else if !is_pressed && self.is_resize_pressed{
                            self.is_resize_pressed = false;
                        }
                        true
                    }
                    // toggle is_help_pressed if the key is pressed
                    KeyCode::KeyH => {
                        self.is_help_pressed = is_pressed;
                        if is_pressed && !self.is_being_helped {
                            self.is_being_helped = true;
                        }
                        else if is_pressed && self.is_being_helped {
                            self.is_help_just_pressed = true;
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

    /// update the objects in the world based off the key presses
    pub fn update_world(&mut self) {
        // if not in the help menu
        if !self.is_being_helped {
            let mut change_occurred = false;
            if self.initialized {
                change_occurred = true;
                self.initialized = false;
            }
            if self.is_increase_pressed {
                self.num_instances += 1;
                change_occurred = true;
            }
            if self.is_decrease_pressed && self.num_instances > 0{
                self.num_instances -= 1;
                change_occurred = !change_occurred;
            }
            
            if self.is_spin {
                change_occurred = true;
                // as the number of instances it takes longer to spin all of them, 
                // so we increase the change according to the number of instances
                self.cur_angle = self.cur_angle + 0.5 + (self.num_instances/200) as f32 + (self.num_instances/1000) as f32;
                if self.cur_angle >= 360.0 {
                    self.cur_angle-=360.0;
                }
            }

            if self.is_color_change && !self.is_color_change_pressed{
                change_occurred = true;
                self.is_color_change_pressed = true;
                self.models[0].change_material();
            } else if !self.is_color_change {
                self.is_color_change_pressed = false;
            }

            if self.is_resize {
                change_occurred = true;

                // increase or decreace the scale depending if the instances are getting bigger or smaller
                if self.is_upscalling {
                    self.cur_scale = self.cur_scale + 0.01;
                    // if we reached the max size, start to decreace the scale
                    if self.cur_scale >= 1.0 {
                        self.is_upscalling = false;
                    }
                } else {
                    self.cur_scale = self.cur_scale - 0.01;
                    // if we reached the min size, start to increase the scale
                    if self.cur_scale <= 0.5 {
                        self.is_upscalling = true;
                    }
                }
            }


            if change_occurred {
                // set up instances
                // this is all our objects
                const SPACE_BETWEEN: f32 = 3.0;

                let num_instances = self.num_instances;
                let mut angle = self.cur_angle;
                let scale: f32 = self.cur_scale;

                // we are making a n*n grid of cubes that are rotated at weird angles
                let instances = (0..num_instances).flat_map(|z| {
                    (0..num_instances).map(move |x| {
                        let x = SPACE_BETWEEN * (x as f32 - num_instances as f32 / 2.0);
                        let z = SPACE_BETWEEN * (z as f32 - num_instances as f32 / 2.0);

                        let position = cgmath::Vector3 { x, y: 0.0, z };
                        
                        if position.is_zero() {
                            angle+= 45.0;
                        }
                        let rotation = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(angle));

                        instance::Instance {
                            position, rotation, scale
                        }
                    })
                }).collect::<Vec<_>>();
                self.models[0].set_instances(instances);
            }
        }
    }

    // creates an instance of a cube with the help menu texture in models[1]
    // and switches the visible models
    pub fn go_to_help(&mut self) {
        
        if self.is_being_helped {
            // set up 1 instance of a cube
            // this will have the help menu texture
            let scale: f32 = 1.0;

            // we are making 1 cube
            let instances = (0..1).flat_map(|z| {
                (0..1).map(move |x| {
                    let x = 0.0;
                    let z = -1.0;
                    let position = cgmath::Vector3 {x, y: 0.0, z};
                    
                    let rotation = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(270.0));
                    

                    instance::Instance {
                        position, rotation, scale
                    }
                })
            }).collect::<Vec<_>>();
            self.models[1].set_instances(instances);
            self.models[0].visible = false;
            self.models[1].visible = true;
        }

        if !self.is_being_helped {
            if self.is_help_just_pressed {
                self.models[1].visible = false;
                self.models[0].visible = true;
                self.is_help_just_pressed = false;
            }
        }
    }
}

pub trait DrawWorld<'a> {
    fn draw_world(&mut self, world: &'a World, camera_bind_group: &'a wgpu::BindGroup);
}

/// set up drawing models for our RenderPass rendering pipeline
impl<'a, 'b> DrawWorld<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_world(&mut self, world: &'b World, camera_bind_group: &'b wgpu::BindGroup) {
        for model in &world.models {
            self.draw_model(model, camera_bind_group);
        }
    }
}