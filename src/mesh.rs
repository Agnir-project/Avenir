// use octree::Octree; TODO
use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    Triangulate,
};
use nalgebra::{Matrix3, Matrix4, Vector2, Vector3};
use rendy::command::{DrawIndexedCommand, QueueId, RenderPassEncoder};
use rendy::factory::Factory;
use rendy::graph::render::*;
use rendy::graph::{
    render::{Layout, SimpleGraphicsPipeline, SimpleGraphicsPipelineDesc},
    GraphContext, NodeBuffer, NodeImage,
};
use rendy::hal;
use rendy::hal::{adapter::PhysicalDevice, device::Device, pso::DescriptorPool};
use rendy::memory::MemoryUsageValue;
use rendy::mesh::{AsVertex, Mesh, Model, PosColorNorm, Position};
use rendy::resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle};
use rendy::shader::{
    Shader, ShaderKind, ShaderSet, ShaderSetBuilder, SourceLanguage, SourceShaderInfo, SpirvShader,
};

#[cfg(feature = "experimental-spirv-reflection")]
use rendy::shader::SpirvReflection;

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
            include_str!("shader.frag"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/shader.frag").into(),
            ShaderKind::Fragment,
            SourceLanguage::GLSL,
            "main",
        ).precompile().unwrap();

    static ref SHADERS: ShaderSetBuilder = ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}

#[cfg(feature = "experimental-spirv-reflection")]
lazy_static::lazy_static! {
    static ref SHADER_REFLECTION: SpirvReflection = SHADERS.reflect().unwrap();
}

const UNIFORM_SIZE: u64 = std::mem::size_of::<UniformArgs>() as u64;
const MAX_INSTANCE: u64 = 1_000_000;
const MODEL_SIZE: u64 = std::mem::size_of::<Model>() as u64 * MAX_INSTANCE;
const INDIRECT_SIZE: u64 = std::mem::size_of::<DrawIndexedCommand>() as u64;

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct UniformArgs {
    model: nalgebra::Matrix3<f32>,
    proj: nalgebra::Perspective3<f32>,
    view: nalgebra::Projective3<f32>,
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct PosColor {
    pos: nalgebra::Vector3<f32>,
    color: nalgebra::Vector4<f32>,
}

#[derive(Debug, Default)]
pub struct PipelineDesc;

pub struct Pipeline<B: hal::Backend> {
    align: u64,
    buffer: Escape<Buffer<B>>,
    sets: Vec<Escape<DescriptorSet<B>>>,
    mesh: Mesh<B>,
    positions: Vec<Matrix3<f32>>,
}

impl<B: hal::Backend> std::fmt::Debug for Pipeline<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Pipeline Test")
    }
}

impl<B> SimpleGraphicsPipelineDesc<B, ()> for PipelineDesc
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

    fn load_shader_set(&self, factory: &mut Factory<B>, _aux: &()) -> rendy::shader::ShaderSet<B> {
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
            push_constants: vec![],
        };
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        _aux: &(),
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
                    size: MODEL_SIZE + UNIFORM_SIZE + INDIRECT_SIZE,
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
                        Some(index)..Some(index + UNIFORM_SIZE),
                    )),
                }));
                sets.push(set);
            }
        }

        let mesh = {
            let cube = genmesh::generators::Cube::new();
            let indices: Vec<_> = genmesh::Vertices::vertices(cube.indexed_polygon_iter())
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

                Mesh::<B>::builder()
                    .with_indices(indices)
                    .with_vertices(vertices)
                    .build(queue, &factory)
                    .unwrap()
        };

        Ok(Pipeline {
            align,
            buffer,
            sets,
            mesh,
            positions: vec![nalgebra::Matrix3::identity()],
        })
    }
}

impl<B> SimpleGraphicsPipeline<B, ()> for Pipeline<B>
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
        _aux: &(),
    ) -> PrepareResult {
        debug!("Pipeline Mesh, Preparing.");

        unsafe {
            // Upload Uniform Parameters
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    (UNIFORM_SIZE + MODEL_SIZE + INDIRECT_SIZE) * index as u64,
                    &[UniformArgs {
                        proj: nalgebra::Perspective3::new(1920. / 1080., 3.14116 / 4.0, 1.0, 200.0),
                        view: nalgebra::Projective3::identity()
                            * nalgebra::Translation3::new(0.0, 0.0, 10.0),
                        model: nalgebra::Matrix3::identity(),
                    }],
                )
                .unwrap();
        };

        unsafe {
            // Upload Index Command
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    ((UNIFORM_SIZE + MODEL_SIZE + INDIRECT_SIZE) * index as u64)
                        + MODEL_SIZE
                        + UNIFORM_SIZE as u64,
                    &[DrawIndexedCommand {
                        index_count: self.mesh.len(),
                        instance_count: 1,
                        first_index: 0,
                        vertex_offset: 0,
                        first_instance: 0,
                    }],
                )
                .unwrap()
        }

        unsafe {
            // Upload positions
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    ((UNIFORM_SIZE + MODEL_SIZE + INDIRECT_SIZE) * index as u64) + UNIFORM_SIZE,
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
        _aux: &(),
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
                std::iter::once((
                    self.buffer.raw(),
                    ((UNIFORM_SIZE + MODEL_SIZE + INDIRECT_SIZE) * index as u64) + UNIFORM_SIZE,
                )),
            );
            encoder.draw_indexed_indirect(
                self.buffer.raw(),
                ((UNIFORM_SIZE + MODEL_SIZE + INDIRECT_SIZE) * index as u64)
                    + UNIFORM_SIZE
                    + MODEL_SIZE,
                1,
                INDIRECT_SIZE as u32,
            );
        }
    }

    fn dispose(mut self, factory: &mut Factory<B>, _aux: &()) {
        info!("Disposing Pipeline Mesh.");
    }
}
