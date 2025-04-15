use bytemuck::{Pod, Zeroable};

use wgpu_dance::model::{RenderVertex, VertexFromMeshIndex};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

unsafe impl Zeroable for Vertex {}
unsafe impl Pod for Vertex {}

impl RenderVertex for Vertex {
    fn buffer_layout_desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use core::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl VertexFromMeshIndex for Vertex {
    fn from_mesh_index(mesh: &tobj::Mesh, i: usize) -> Self {
        Vertex {
            position: [
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
            ],
            tex_coords: [mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1]],
            normal: [
                mesh.normals[i * 3],
                mesh.normals[i * 3 + 1],
                mesh.normals[i * 3 + 2],
            ],
        }
    }
}
