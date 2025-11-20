/// File to represent the overall state of the current window

mod camera;
mod camera_controller;
mod world;
mod mouse_grabber;

use std::rc::Rc;

use mouse_grabber::{MouseGrabber};
use wgpu::util::DeviceExt;
use winit::{event::{MouseScrollDelta, WindowEvent}, window::Window};

use world::{instance::InstanceRaw, model::{self, Vertex}, texture, DrawWorld, World};

/// structure to store the sate of the window/frame
pub struct State<'a> {
    pub size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'a>,
    device: Rc<wgpu::Device>,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// describe how we render things
    render_pipeline: wgpu::RenderPipeline,
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    pub camera_controller: camera_controller::CameraController,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    world: World,
    mouse_grabber: MouseGrabber,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: &'a Window,
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code

    /// create a new state object for a window
    pub async fn new(window: &'a Window) -> State<'a> {
        // set the size
        let size = window.inner_size();

        // The instance represents how we work with all wgpu stuff
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        
        // set up the surface our GPU writes to
        let surface = instance.create_surface(window).unwrap();

        // Set up our adapter to our GPU
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        // Set up our interface with our GPU to interact with it
        let (device_obj, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: Default::default(),
            },
            None, // Trace path
        ).await.unwrap();

        // put device onto the heap so we can share ownership
        let device = Rc::new(device_obj);

        // returns what the surface can do/our available operations with the present GPU
        let surface_caps = surface.get_capabilities(&adapter);

        // configure our surface to be an sRGB surface texture
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // Configure our surface size and refresh rate
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        // Textures:
        // define how binding are laid out for the fragment shader
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        
        // setting up the camera
        // Here is the user friendly info
        let camera = camera::Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        // This stores the main camera matrix
        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        // buffer to send camera data to our GPU
        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        // set up a controller to control the camera
        let camera_controller = camera_controller::CameraController::new(0.05);

        // set up the camera bind group memory layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        // now make the camera bind group layout
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });


        // create our depth texture
        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");
    
        // creating the shaders
        // We are going to use the functions from the shader.wgsl for our shaders
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // setup the layout for the render pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[  // this is where we register our bind layouts
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState { // Specify that we use the vertex function from shader.wgsl
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    model::ModelVertex::desc(),
                    InstanceRaw::desc(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState { // Specify that we use the fragment vertex function from shader.wgsl
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { // setup a color output for the surface
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState { // handle depth and when things are behind each other
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // draw front to back
                stencil: wgpu::StencilState::default(), 
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1, // only 1 sample because multisampling is a bit complex
                mask: !0, // use all the samples
                alpha_to_coverage_enabled: false, // we won't do aliasing either
            },
            multiview: None, // we also wont be using array textures
            cache: None, // we dont need caching either
        });

        // establish the world with all its models and instances
        let world = World::new(&device, &queue, &texture_bind_group_layout).await;

        // setup something to keep our mouse centered
        let mouse_grabber = MouseGrabber { mouse_locked: false };
        

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            depth_texture,
            world,
            mouse_grabber,
        }
    }
    
    /// get the current window
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// resize the window
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }

        self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
    }

    /// Handle user input
    /// 
    /// Returns true if it successfully handled the user input
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        let mut result = self.mouse_grabber.process_events(event, self.window);
        result = self.camera_controller.process_events(event) || result;
        result = self.world.process_events(event) || result;
        return result;
    }


    /// Handle mouse movement event
    pub fn process_mouse_movement(&mut self, delta_x: f64, delta_y: f64) {
        if self.mouse_grabber.mouse_locked {
            self.camera_controller.process_mouse(delta_x, delta_y);
        }
        self.mouse_grabber.process_mouse(self.window, &self.size);
    }

    // Handle mouse wheel event
    pub fn process_mouse_wheel(&mut self, delta: &MouseScrollDelta) {
        if self.mouse_grabber.mouse_locked {
            match delta {
                MouseScrollDelta::LineDelta(_, scroll) => self.camera_controller.process_mouse_wheel(*scroll, &mut self.camera),
                MouseScrollDelta::PixelDelta(physical_position) => (),
            };
    
        }
    }

    /// update various objects in the program
    pub fn update(&mut self) {
        self.world.update_world();
        self.world.go_to_help();
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    /// render objects to the screen
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // grab frame to render to
        let output = self.surface.get_current_texture()?;
        
        // grab to view to draw onto
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create an encoder to send commands to the GPU
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
    
        // put this in a borrow block since render pass will borrow the encoder
        // When this section is done rust will know to release the mutable borrow
        // allowing us to perform encoder.finish()
        {
            // for now we are just setting the screen to a constant color
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, // render to the view from earlier
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { // clear the screen to a color
                            r: 0.5,
                            g: 0.1,
                            b: 0.5,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { // make sure pixels are drawn back to front
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Use our pipeline we defined
            render_pass.set_pipeline(&self.render_pipeline);

            // Here we are drawing all the instances
            // in the future we could optimize this to only draw the instances on screen
            render_pass.draw_world(&self.world, &self.camera_bind_group);
 
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();


        Ok(())
    }
}