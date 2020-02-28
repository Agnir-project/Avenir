use rendy::command::{QueueId, RenderPassEncoder};
use rendy::factory::Factory;
use rendy::graph::{
    render::{Layout, SimpleGraphicsPipeline, SimpleGraphicsPipelineDesc},
    GraphContext, NodeBuffer, NodeImage,
};
use rendy::hal;
use rendy::resource::{DescriptorSetLayout, Handle};

#[derive(Debug, Default)]
pub struct PipelineDesc;

pub struct Pipeline<B: hal::Backend> {
    pool: B::DescriptorPool,
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
        vec![]
    }

    fn load_shader_set(&self, factory: &mut Factory<B>, aux: &()) -> rendy::shader::ShaderSet<B> {
        Default::default()
    }

    fn layout(&self) -> Layout {
        Layout {
            sets: vec![],
            push_constants: vec![],
        }
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        aux: &(),
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Self::Pipeline, rendy::core::hal::pso::CreationError> {
        Err(rendy::core::hal::pso::CreationError::Other)
    }
}

impl<B> SimpleGraphicsPipeline<B, ()> for Pipeline<B>
where
    B: hal::Backend,
{
    type Desc = PipelineDesc;

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        encoder: RenderPassEncoder<'_, B>,
        index: usize,
        aux: &(),
    ) {
    }

    fn dispose(self, factory: &mut Factory<B>, aux: &()) {}
}
