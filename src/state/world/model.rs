/// Represent a model and how its rendered.
use std::{ops::Range, rc::Rc};

use wgpu::util::DeviceExt;

use super::{instance::{self, Instance}, texture};

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {

    /// describe memory layout for a vertex
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0, // offset from start
                    shader_location: 0,  // position field
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1, // texture field  
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Represent a model
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub visible: bool,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    /// device this model is rendered with
    device: Rc<wgpu::Device>,
}

impl Model {
    /// make a new model
    pub fn new(meshes: Vec<Mesh>, materials: Vec<Material>, device: Rc<wgpu::Device>) -> Model{
        // No instances to start
        let instances = Vec::new();

        let instance_data = instances.iter().map(instance::Instance::to_raw).collect::<Vec<_>>();

        // create instance buffer
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        Self {
            meshes,
            materials,
            visible:true,
            instances,
            instance_buffer,
            device,
        }
    }

    /// set instances to something
    pub fn set_instances(&mut self, instances: Vec<Instance>) {

        let instance_data = instances.iter().map(instance::Instance::to_raw).collect::<Vec<_>>();
        
        // create instance buffer
        let instance_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        self.instances = instances;
        self.instance_buffer = instance_buffer;
    }

    /// Add a new instance
    pub fn add_instances(&mut self, instance: Instance) {
        self.instances.push(instance);

        let instance_data = self.instances.iter().map(instance::Instance::to_raw).collect::<Vec<_>>();

        // create instance buffer
        let instance_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        self.instance_buffer = instance_buffer;
    }

    pub fn change_material(&mut self){
        self.meshes[0].material = self.meshes[0].material + 1;
        if self.materials.len() <=self.meshes[0].material {
            self.meshes[0].material = 0;
        }
    }
}

/// represent the material for a model
pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

/// represent the mesh for a model
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}


/// interface for drawing our models
pub trait DrawModel<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh, material: &'a Material, camera_bind_group: &'a wgpu::BindGroup);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model(&mut self, model: &'a Model, camera_bind_group: &'a wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

/// set up drawing models for our RenderPass rendering pipeline
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    /// Draw a mesh to screen
    /// 
    /// Args:
    ///     mesh: mesh to draw
    ///     material: material to draw onto object
    ///     camera_bind_group: camera group to render in
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, camera_bind_group: &'b wgpu::BindGroup) {

        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group);
    }

    /// Draws several instances of a model
    /// 
    /// Args:
    ///     mesh: mesh to draw
    ///     material: material to drawn onto the object
    ///     instances: list of which instances to draw
    ///     camera_bind_group: camera information to render into the group
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
    ){
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    /// Draw a model using its texture to a camera
    fn draw_model(&mut self, model: &'b Model, camera_bind_group: &'b wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..model.instances.len() as u32, camera_bind_group);
    }

    /// Draw a model using its texture to a camera
    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        if model.visible {
            for mesh in &model.meshes {
                let material = &model.materials[mesh.material];
                self.set_vertex_buffer(1, model.instance_buffer.slice(..));
                self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group);
            }
        }
    }
}