use crate::Backend;
use genmesh::{Vertices, generators::IndexedPolygon, generators::SharedVertex};
use std::borrow::Cow;
use rendy::{
    command::{
        CommandBuffer, CommandPool, Compute, DrawCommand, DrawIndexedCommand, ExecutableState,
        Families, Family, MultiShot, PendingState, QueueId, RenderPassEncoder, SimultaneousUse,
        Submit,
    },
    core::EnabledBackend,
    factory::{BufferState, Config, Factory},
    frame::Frames,
    graph::{
        gfx_acquire_barriers, gfx_release_barriers,
        render::{
            Layout, PrepareResult, RenderGroupBuilder, SetLayout, SimpleGraphicsPipeline,
            SimpleGraphicsPipelineDesc,
        },
        BufferAccess, Graph, GraphBuilder, GraphContext, Node, NodeBuffer, NodeBuildError,
        NodeDesc, NodeImage, NodeSubmittable,
    },
    hal::{self, adapter::PhysicalDevice, device::Device, pso::CreationError},
    init::{
        winit::{
            dpi::{LogicalSize, PhysicalSize},
            event::{Event, VirtualKeyCode, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::{Window, WindowBuilder},
        },
        AnyWindowedRendy,
    },
    memory::Dynamic,
    mesh::{Indices, Mesh, Model, PosColorNorm, AsVertex},
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    shader::{Shader, ShaderKind, ShaderSet, SourceLanguage, SourceShaderInfo, SpirvShader},
    wsi::Surface,
};

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct Light {
    pub pos: nalgebra::Vector3<f32>,
    pub pad: f32,
    pub intensity: f32,
}

#[derive(Debug)]
pub struct Camera {
    pub view: nalgebra::Projective3<f32>,
    pub proj: nalgebra::Perspective3<f32>,
}

#[derive(Debug)]
pub struct Scene<B: hal::Backend> {
    pub camera: Camera,
    pub object_mesh: Option<Mesh<B>>,
    pub objects: Vec<nalgebra::Transform3<f32>>,
    pub lights: Vec<Light>,
}

impl<'a, B: hal::Backend> Scene<B> {
    pub fn add_cube(&mut self, queue: QueueId, factory: &Factory<B>) {
        let cube = genmesh::generators::Cube::new();
        let indices: Vec<_> = Vertices::vertices(cube.indexed_polygon_iter())
            .map(|i| i as u32)
            .collect();
        let vertices: Vec<_> = cube
            .shared_vertex_iter()
            .map(|v| PosColorNorm {
                position: v.pos.into(),
                color: [
                    (v.pos.x + 1.0) / 2.0,
                    (v.pos.y + 1.0) / 2.0,
                    (v.pos.z + 1.0) / 2.0,
                    1.0,
                ]
                .into(),
                normal: v.normal.into(),
            })
            .collect();

        self.set_object_mesh(&indices[..], &vertices[..], queue, factory);
    }

    pub fn add_sphere(&mut self, queue: QueueId, factory: &Factory<B>) {
        let icosphere = genmesh::generators::IcoSphere::subdivide(4);
        let indices: Vec<_> = Vertices::vertices(icosphere.indexed_polygon_iter())
            .map(|i| i as u32)
            .collect();
        let vertices: Vec<_> = icosphere
            .shared_vertex_iter()
            .map(|v| PosColorNorm {
                position: v.pos.into(),
                color: [
                    (v.pos.x + 1.0) / 2.0,
                    (v.pos.y + 1.0) / 2.0,
                    (v.pos.z + 1.0) / 2.0,
                    1.0,
                ]
                .into(),
                normal: v.normal.into(),
            })
            .collect();

        self.set_object_mesh(&indices[..], &vertices[..], queue, factory);
    }

    pub fn set_object_mesh<I, V, D>(
        &mut self,
        indices: I,
        vertices: D,
        queue: QueueId,
        factory: &Factory<B>,
    ) where
        I: Into<Indices<'a>>,
        V: AsVertex + 'a,
        D: Into<Cow<'a, [V]>>,
    {
        self.object_mesh = Some(
            Mesh::<Backend>::builder()
                .with_indices(indices)
                .with_vertices(vertices)
                .build(queue, factory)
                .unwrap(),
        );
    }
}
