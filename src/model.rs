use std::{
    io::{BufReader, Cursor},
    ops::Range,
};

use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, Buffer, Device};

use crate::{
    resource::{load_string, load_texture},
    texture::Texture,
};

pub trait RenderVertex: Zeroable + Pod {
    fn buffer_layout_desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[derive(Debug, Clone)]
pub struct Model<V: RenderVertex> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<Buffer>,
    pub index_buffer: Option<Buffer>,
    pub label: String,
}

impl<V: RenderVertex> Model<V> {
    pub fn new(vertices: &[V], indices: &[u32], label: &str) -> Self {
        Self {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
            vertex_buffer: None,
            index_buffer: None,
            label: label.to_string(),
        }
    }

    pub fn alloc_buffer(&mut self, device: &Device) {
        assert!(self.vertex_buffer.is_none());
        assert!(self.index_buffer.is_none());

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} vertex buffer", self.label)),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} index buffer", self.label)),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.vertex_buffer.replace(vertex_buffer);
        self.index_buffer.replace(index_buffer);
    }
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct MeshModel {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub trait VertexFromMeshIndex {
    fn from_mesh_index(mesh: &tobj::Mesh, index: usize) -> Self;
}

impl MeshModel {
    pub async fn load_model<V: VertexFromMeshIndex + RenderVertex>(
        file_name: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Self> {
        let obj_text = load_string(file_name).await?;
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (models, obj_materials) = tobj::load_obj_buf_async(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| async move {
                let mat_text = load_string(&p).await.unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
            },
        )
        .await?;

        let mut materials = Vec::new();
        for m in obj_materials? {
            let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
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

            materials.push(Material {
                name: m.name,
                diffuse_texture,
                bind_group,
            })
        }

        let meshes = models
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| V::from_mesh_index(&m.mesh, i))
                    .collect::<Vec<_>>();

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", file_name)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", file_name)),
                    contents: bytemuck::cast_slice(&m.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                Mesh {
                    name: file_name.to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    material: m.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        Ok(MeshModel { meshes, materials })
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model(&mut self, model: &'a MeshModel, camera_bind_group: &'a wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'a MeshModel,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(1, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, &material.bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
    fn draw_model(&mut self, model: &'b MeshModel, camera_bind_group: &'b wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, camera_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b MeshModel,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group);
        }
    }
}
