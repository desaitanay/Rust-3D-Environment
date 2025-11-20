/// help load files and objects

use std::{io::{BufReader, Cursor}, path::Path, rc::Rc};

use wgpu::util::DeviceExt;

use super::{model, texture};

/// function to load string data from a file
pub async fn load_string(file_name: &dyn AsRef<Path>) -> anyhow::Result<String> {
    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let txt = std::fs::read_to_string(path)?;

    Ok(txt)
}

/// Function to load binary data from a file
pub async fn load_binary(file_name: &dyn AsRef<Path>) -> anyhow::Result<Vec<u8>> {
    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let data = std::fs::read(path)?;

    Ok(data)
}

/// function to load a texture from file
/// 
/// Args:
///     file_name: path to texture
///     device: device to load onto
///     queue: command queue for device
pub async fn load_texture(
    file_name: &dyn AsRef<Path>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name.as_ref().file_name().unwrap().to_str().unwrap())
}


/// function to load a model from a .obj file
///
/// Args:
///     file_name: name of file/ path to file
///     device: graphics/compute device to load into
///     queue: command queue to device
///     layout: model memory layout
pub async fn load_model(
    file_name: &str,
    device: Rc<wgpu::Device>,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    // read file
    let model_dir = Path::new(file_name).parent().unwrap();
    // .as_os_str().to_str().unwrap();
    let obj_text = load_string(&file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    // use tobj to get the models and materials
    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&model_dir.join(p)).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();
    // load all the textures for all the materials and create their bindings
    for m in obj_materials? {
        let diffuse_texture = load_texture(&model_dir.join(m.diffuse_texture), &device, queue).await?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        materials.push(model::Material {
            name: m.name,
            diffuse_texture,
            bind_group,
        })
    }

    // load all the meshes as vertexes
    let meshes = models
        .into_iter()
        .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    if m.mesh.normals.is_empty(){  // if the normals aren't specified we use [0, 0, 0]
                        model::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                            normal: [0.0, 0.0, 0.0],
                        }
                    }else{  // otherwise we grab the normals from the mesh
                        model::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                            normal: [
                                m.mesh.normals[i * 3],
                                m.mesh.normals[i * 3 + 1],
                                m.mesh.normals[i * 3 + 2],
                            ],
                        }
                    }
                })
                .collect::<Vec<_>>();

            // now we create a vertex buffer to represent the possible vertexes for the model
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            // now we create an index buffer for the model
            // this is to reduce the amount of vertices we have my reindex them over and over again
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(model::Model::new(meshes, materials, device))
}

/// Tests for resources
#[cfg(test)]
mod tests {
    use super::*;

    /// Test that we can properly read text from file
    #[test]
    fn test_load_text() {

        // type is inferred
        let foo = 67;

        // can cast when necessary
        let bar = foo as u8 as char;
        let baz = 32 as u16;

        let text = tokio_test::block_on(load_string(&"test_files/hello_world.txt")).unwrap();

        assert_eq!(text, "Hello World!");
    }

    /// Test that we can properly read bytes from file
    #[test]
    fn test_load_binary() {
        let text = tokio_test::block_on(load_binary(&"test_files/hello_world.txt")).unwrap();

        assert_eq!(text, vec![72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 33]);
    }
}