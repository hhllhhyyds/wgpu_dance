use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, Buffer, Device};

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
