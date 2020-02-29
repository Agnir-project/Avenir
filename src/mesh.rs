// use octree::Octree; TODO
use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    Triangulate,
};
use nalgebra::{Matrix3, Matrix4, Vector2, Vector3};
use rendy::command::{QueueId, RenderPassEncoder};
use rendy::factory::Factory;
use rendy::graph::render::*;
use rendy::graph::{
    render::{Layout, SimpleGraphicsPipeline, SimpleGraphicsPipelineDesc},
    GraphContext, NodeBuffer, NodeImage,
};
use rendy::hal;
use rendy::hal::{device::Device, pso::DescriptorPool};
use rendy::memory::MemoryUsageValue;
use rendy::mesh::{AsVertex, Mesh, Position};
use rendy::resource::{Buffer, BufferInfo, DescriptorSetLayout, Escape, Handle};
use rendy::shader::{
    Shader, ShaderKind, ShaderSet, ShaderSetBuilder, SourceLanguage, SourceShaderInfo, SpirvShader,
};

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

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct UniformArgs {
    model: nalgebra::Matrix4<f32>,
    proj: nalgebra::Matrix4<f32>,
    view: nalgebra::Matrix4<f32>,
}

#[derive(Debug, Default)]
pub struct PipelineDesc;

pub struct Pipeline<B: hal::Backend> {
    octrees: Vec<Mesh<B>>,
    set: B::DescriptorSet,
    pool: B::DescriptorPool,
    buffer: Escape<Buffer<B>>,
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
        Vec<rendy::core::hal::pso::Element<rendy::core::hal::format::Format>>,
        rendy::core::hal::pso::ElemStride,
        rendy::core::hal::pso::VertexInputRate,
    )> {
        vec![Position::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)]
        // TODO: understand
    }

    fn load_shader_set(&self, factory: &mut Factory<B>, _aux: &()) -> rendy::shader::ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn layout(&self) -> Layout {
        Layout {
            // Shader layout
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
        }
    }

    fn build<'a>(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        _aux: &(),
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Self::Pipeline, rendy::core::hal::pso::CreationError> {
        // Generate vertices from octree. -- TODO: Understand
        let cube = genmesh::generators::Cube::new();
        let cube_vertices: Vec<_> = cube
            .shared_vertex_iter()
            .map(|v| Position(v.pos.into()))
            .collect();
        let cube_flattened_vertices: Vec<_> =
            genmesh::Vertices::vertices(cube.indexed_polygon_iter().triangulate())
                .map(|i| cube_vertices[i])
                .collect();
        let cube = Mesh::<B>::builder()
            .with_vertices(&cube_flattened_vertices[..])
            .build(queue, factory)
            .unwrap();

        let mut pool = unsafe {
            factory
                .create_descriptor_pool(
                    3,
                    vec![hal::pso::DescriptorRangeDesc {
                        ty: hal::pso::DescriptorType::UniformBuffer,
                        count: 3,
                    }],
                    hal::pso::DescriptorPoolCreateFlags::empty(),
                )
                .unwrap()
        };

        let buffer = factory
            .create_buffer(
                BufferInfo {
                    size: 3 * 100000, // TODO: Change size
                    usage: hal::buffer::Usage::UNIFORM,
                },
                MemoryUsageValue::Dynamic,
            )
            .unwrap();

        let set = unsafe {
            let set = pool.allocate_set(&set_layouts[0].raw()).unwrap();
            factory.write_descriptor_sets(vec![hal::pso::DescriptorSetWrite {
                set: &set,
                binding: 0,
                array_offset: 0,
                descriptors: Some(hal::pso::Descriptor::Buffer(
                    buffer.raw(),
                    Some(100000 * 3)..Some(100000 * 4),
                )),
            }]);
            set
        };

        Ok(Pipeline {
            set,
            pool,
            buffer,
            octrees: vec![cube],
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
        unsafe {
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    100000 * index as u64,
                    &[UniformArgs {
                        proj: Matrix4::identity(),
                        view: Matrix4::identity(),
                        model: Matrix4::identity(),
                    }],
                )
                .unwrap();
        };

        // Prepare the buffer and the uniforms.
        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        _index: usize,
        _aux: &(),
    ) {
        for (i, tree) in self.octrees.iter().enumerate() {
            tree.bind(i as u32, &[Position::vertex()], &mut encoder)
                .unwrap();
        }

        unsafe {
            encoder.bind_graphics_descriptor_sets(layout, 0, vec![&self.set], std::iter::empty());
            encoder.draw(0..36, 0..1);
        }
        // Bind descriptor sets and draw.
    }

    fn dispose(mut self, factory: &mut Factory<B>, _aux: &()) {
        // Free descriptor pool.
        // Free rest.
        unsafe {
            self.pool.reset();
            factory.destroy_descriptor_pool(self.pool);
        }
    }
}
