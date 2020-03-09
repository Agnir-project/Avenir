use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    MapToVertices, Triangulate, Vertices,
};
use nalgebra::{Translation3, Matrix4, Matrix3, Perspective3, Point3, Projective3, Vector3, Isometry3};
use rendy::command::{DrawIndexedCommand, QueueId, RenderPassEncoder};
use rendy::factory::Factory;
use rendy::graph::render::*;
use rendy::graph::{
    render::{Layout, SimpleGraphicsPipeline, SimpleGraphicsPipelineDesc},
    GraphContext, NodeBuffer, NodeImage,
};
use rendy::hal;
use rendy::hal::{adapter::PhysicalDevice, device::Device};

use rendy::mesh::{AsVertex, Mesh, Model, PosColorNorm};
use rendy::resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle};
use rendy::shader::{
    Shader, ShaderKind, ShaderSet, ShaderSetBuilder, SourceLanguage, SourceShaderInfo, SpirvShader,
};
use crate::camera::Camera;
use std::mem::size_of;

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("../shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
            include_str!("../shader.frag"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/shader.frag").into(),
            ShaderKind::Fragment,
            SourceLanguage::GLSL,
            "main",
        ).precompile().unwrap();

    static ref SHADERS: ShaderSetBuilder = ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();

    static ref CUBE: genmesh::generators::Cone = genmesh::generators::Cone::new(10);

    static ref CUBE_INDICES: Vec<u32> = genmesh::Vertices::vertices(CUBE.indexed_polygon_iter())
        .map(|i| i as u32)
        .collect();

    static ref CUBE_VERTICES: Vec<PosColorNorm> = CUBE.shared_vertex_iter()
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
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct UniformArgs {
    pub proj: Matrix4<f32>,
    pub view: Matrix4<f32>,
}

#[derive(Debug, Default)]
pub struct PipelineDesc;

pub struct Pipeline<B: hal::Backend> {
    align: u64,
    buffer: Escape<Buffer<B>>,
    sets: Vec<Escape<DescriptorSet<B>>>,
    mesh: Mesh<B>,
    positions: Vec<nalgebra::Transform3<f32>>,
}

const MAX_OBJECTS: usize = 100;
const UNIFORM_SIZE: u64 = size_of::<UniformArgs>() as u64;
const MODELS_SIZE: u64 = size_of::<Model>() as u64 * MAX_OBJECTS as u64;
const INDIRECT_SIZE: u64 = size_of::<DrawIndexedCommand>() as u64;

fn iceil(value: u64, scale: u64) -> u64 {
    ((value - 1) / scale + 1) * scale
}

fn buffer_frame_size(align: u64) -> u64 {
    iceil(UNIFORM_SIZE + MODELS_SIZE + INDIRECT_SIZE, align)
}

fn uniform_offset(index: usize, align: u64) -> u64 {
    buffer_frame_size(align) * index as u64
}

fn models_offset(index: usize, align: u64) -> u64 {
    uniform_offset(index, align) + UNIFORM_SIZE
}

fn indirect_offset(index: usize, align: u64) -> u64 {
    models_offset(index, align) + MODELS_SIZE
}

impl<B: hal::Backend> std::fmt::Debug for Pipeline<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Pipeline Test")
    }
}

impl<B> SimpleGraphicsPipelineDesc<B, Camera> for PipelineDesc
where
    B: hal::Backend,
{
    type Pipeline = Pipeline<B>;

    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        // Set the vertices for the vertex shader.
        return vec![
            PosColorNorm::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex),
            Model::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Instance(1)),
        ];
    }

    fn load_shader_set(
        &self,
        factory: &mut Factory<B>,
        _aux: &Camera,
    ) -> rendy::shader::ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn layout(&self) -> Layout {
        return Layout {
            sets: vec![SetLayout {
                bindings: vec![hal::pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: hal::pso::DescriptorType::UniformBuffer,
                    count: 1,
                    stage_flags: hal::pso::ShaderStageFlags::VERTEX,
                    immutable_samplers: false,
                }],
            }],
            push_constants: Vec::new(),
        };
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        _aux: &Camera,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Self::Pipeline, hal::pso::CreationError> {
        let frames = ctx.frames_in_flight as _;
        let align = factory
            .physical()
            .limits()
            .min_uniform_buffer_offset_alignment;

        let buffer = factory
            .create_buffer(
                BufferInfo {
                    size: buffer_frame_size(align) * frames as u64,
                    usage: hal::buffer::Usage::UNIFORM
                        | hal::buffer::Usage::INDIRECT
                        | hal::buffer::Usage::VERTEX,
                },
                rendy::memory::Dynamic,
            )
            .unwrap();

        let mut sets = Vec::new();

        for index in 0..frames {
            unsafe {
                let set = factory
                    .create_descriptor_set(set_layouts[0].clone())
                    .unwrap();
                factory.write_descriptor_sets(Some(hal::pso::DescriptorSetWrite {
                    set: set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: Some(hal::pso::Descriptor::Buffer(
                        buffer.raw(),
                        Some(uniform_offset(index, align))
                            ..Some(uniform_offset(index, align) + UNIFORM_SIZE),
                    )),
                }));
                sets.push(set);
            }
        }

        let mesh = Mesh::<B>::builder()
            .with_vertices(&(*CUBE_VERTICES)[..])
            .with_indices(&(*CUBE_INDICES)[..])
            .build(queue, &factory)
            .unwrap();

        let positions: Vec<nalgebra::Transform3<f32>> = (0..MAX_OBJECTS)
            .map(|i| {
                nalgebra::Transform3::identity()
                    * nalgebra::Translation3::new(i as f32, i as f32, i as f32)
            })
            .collect();

        Ok(Pipeline {
            align,
            buffer,
            sets,
            mesh,
            positions,
        })
    }
}

impl<B> SimpleGraphicsPipeline<B, Camera> for Pipeline<B>
where
    B: hal::Backend,
{
    type Desc = PipelineDesc;

    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        index: usize,
        aux: &Camera,
    ) -> PrepareResult {
        debug!("Pipeline Mesh, Preparing {}.", index);

        unsafe {
            // Upload Uniform Parameters
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    uniform_offset(index, self.align) as u64,
                    &[UniformArgs {
                        proj: aux.proj.to_homogeneous(),
                        view: aux.view.inverse().to_homogeneous(),
                    }],
                )
                .unwrap();
        };

        let command = DrawIndexedCommand {
            index_count: self.mesh.len(),
            instance_count: self.positions.len() as u32,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        };

        unsafe {
            // Upload Index Command
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    indirect_offset(index, self.align),
                    &[command],
                )
                .unwrap()
        }

        unsafe {
            // Upload positions
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    models_offset(index, self.align),
                    &self.positions[..],
                )
                .unwrap()
        }

        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _aux: &Camera,
    ) {
        debug!("Pipeline Mesh, Drawing index: {}.", index);

        unsafe {
            encoder.bind_graphics_descriptor_sets(
                layout,
                0,
                Some(self.sets[index].raw()),
                std::iter::empty(),
            );

            let vertex = [PosColorNorm::vertex()];

            self.mesh.bind(0, &vertex, &mut encoder).unwrap();

            encoder.bind_vertex_buffers(
                1,
                std::iter::once((self.buffer.raw(), models_offset(index, self.align))),
            );
            encoder.draw_indexed_indirect(
                self.buffer.raw(),
                indirect_offset(index, self.align),
                1,
                INDIRECT_SIZE as u32,
            );
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &Camera) {
        info!("Disposing Pipeline Mesh.");
    }
}
