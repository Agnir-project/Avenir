use gfx_hal::pass::Subpass;
use gfx_hal::pso::AttributeDesc;
use gfx_hal::pso::BakedStates;
use gfx_hal::pso::BasePipeline;
use gfx_hal::pso::BlendDesc;
use gfx_hal::pso::DepthStencilDesc;
use gfx_hal::pso::DescriptorPool;
use gfx_hal::pso::DescriptorRangeDesc;
use gfx_hal::pso::DescriptorSetLayoutBinding;
use gfx_hal::pso::EntryPoint;
use gfx_hal::pso::GraphicsPipelineDesc;
use gfx_hal::pso::GraphicsShaderSet;
use gfx_hal::pso::InputAssemblerDesc;
use gfx_hal::pso::PipelineCreationFlags;
use gfx_hal::pso::Rasterizer;
use gfx_hal::pso::ShaderStageFlags;
use gfx_hal::pso::Specialization;
use gfx_hal::pso::VertexBufferDesc;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::rc::Rc;

use crate::shader_utils::ShaderUtils;
use crate::utils::{Build, With};
use gfx_hal::window::Extent2D;
use gfx_hal::Backend;
use gfx_hal::Device;

pub struct ShaderEntry<B: Backend<Device = D>, D: Device<B>> {
    shader_module: B::ShaderModule,
    shader_type: shaderc::ShaderKind,
    _device: std::marker::PhantomData<D>,
}

impl<B: Backend<Device = D>, D: Device<B>> ShaderEntry<B, D> {
    fn new(shader_module: B::ShaderModule, shader_type: shaderc::ShaderKind) -> Self {
        ShaderEntry {
            shader_module,
            shader_type,
            _device: std::marker::PhantomData,
        }
    }

    fn compute_entry(&self) -> EntryPoint<B> {
        EntryPoint {
            entry: "main",
            module: &self.shader_module,
            specialization: Specialization {
                constants: &[],
                data: &[],
            },
        }
    }
}

fn vec_shader_entry_into_graphicset<'a, B: Backend<Device = D>, D: Device<B>>(
    from: &'a [ShaderEntry<B, D>],
) -> Result<GraphicsShaderSet<'a, B>, &'static str> {
    let vertex_idx = from
        .iter()
        .position(|elem| elem.shader_type == shaderc::ShaderKind::Vertex)
        .ok_or("No vertex shader found.")?;

    let fragment = from
        .iter()
        .position(|elem| elem.shader_type == shaderc::ShaderKind::Fragment)
        .map(|e| from[e].compute_entry());

    Ok(GraphicsShaderSet {
        vertex: from[vertex_idx].compute_entry(),
        hull: None,
        domain: None,
        geometry: None,
        fragment,
    })
}

pub struct Pipeline<B: Backend<Device = D>, D: Device<B>> {
    pub descriptor_set: B::DescriptorSet,
    pub pipeline_layout: B::PipelineLayout,
    pub graphics_pipeline: B::GraphicsPipeline,
}

pub struct PipelineBuilder<'a, B: Backend<Device = D>, D: Device<B>> {
    base_pipeline: BasePipeline<'a, B::GraphicsPipeline>,
    compiler: shaderc::Compiler,
    device: &'a mut D,
    extent: Extent2D,
    render_pass: &'a B::RenderPass,
    pipeline_creation_flags: PipelineCreationFlags,
    attributes_desc: Vec<AttributeDesc>,
    descriptor_set_layout_binding: Vec<DescriptorSetLayoutBinding>,
    descriptor_range_desc: Vec<DescriptorRangeDesc>,
    immutables_sampler: Vec<B::Sampler>,
    shader_entries: Vec<ShaderEntry<B, D>>,
    vertex_buffers: Vec<VertexBufferDesc>,
    input_assembler_desc: Option<InputAssemblerDesc>,
    rasterizer: Option<Rasterizer>,
    depth_stencil_desc: Option<DepthStencilDesc>,
    blender_desc: Option<BlendDesc>,
    baked_states: Option<BakedStates>,
}

impl<'a, B, D> PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    pub fn new(
        device: &'a mut D,
        extent: Extent2D,
        render_pass: &'a B::RenderPass,
    ) -> Result<Self, &'static str> {
        let compiler = shaderc::Compiler::new().ok_or("shaderc not found!")?;
        Ok(PipelineBuilder {
            shader_entries: vec![],
            compiler,
            device,
            extent,
            render_pass,
            attributes_desc: vec![],
            descriptor_set_layout_binding: vec![],
            descriptor_range_desc: vec![],
            immutables_sampler: vec![],
            vertex_buffers: vec![],
            input_assembler_desc: None,
            rasterizer: None,
            depth_stencil_desc: None,
            blender_desc: None,
            baked_states: None,
            base_pipeline: BasePipeline::None,
            pipeline_creation_flags: PipelineCreationFlags::empty(),
        })
    }

    pub fn with_fragment(
        mut self,
        shader_source: &str,
        entry: &'static str,
    ) -> Result<Self, &'static str> {
        let module = ShaderUtils::<B, D>::fragment_to_module(
            &self.device,
            &mut self.compiler,
            shader_source,
            entry,
        )?;

        self.shader_entries
            .push(ShaderEntry::new(module, shaderc::ShaderKind::Fragment));
        Ok(self)
    }

    pub fn with_vertex(
        mut self,
        shader_source: &str,
        entry: &'static str,
    ) -> Result<Self, &'static str> {
        let module = ShaderUtils::<B, D>::vertex_to_module(
            &self.device,
            &mut self.compiler,
            shader_source,
            entry,
        )?;

        self.shader_entries
            .push(ShaderEntry::new(module, shaderc::ShaderKind::Vertex));
        Ok(self)
    }
}

impl<'a, B, D> Build<Result<Pipeline<B, D>, &'static str>> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn build(self) -> Result<Pipeline<B, D>, &'static str> {
        let descriptor_set_layouts: Vec<<B as Backend>::DescriptorSetLayout> = vec![unsafe {
            self.device
                .create_descriptor_set_layout(
                    &self.descriptor_set_layout_binding[..],
                    &self.immutables_sampler[..],
                )
                .map_err(|_| "Couldn't make a DescriptorSetLayout")?
        }];
        let mut descriptor_pool = unsafe {
            self.device
                .create_descriptor_pool(1, &self.descriptor_range_desc[..])
                .map_err(|_| "Couldn't create a descriptor pool!")?
        };
        let descriptor_set = unsafe {
            descriptor_pool
                .allocate_set(&descriptor_set_layouts[0])
                .map_err(|_| "Couldn't make a Descriptor Set!")?
        };
        let push_constants = Vec::<(ShaderStageFlags, core::ops::Range<u32>)>::new();
        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&descriptor_set_layouts, push_constants)
                .map_err(|_| "Couldn't create a pipeline layout")?
        };
        let graphics_pipeline = {
            let desc = GraphicsPipelineDesc {
                shaders: vec_shader_entry_into_graphicset(&self.shader_entries)?,
                rasterizer: self.rasterizer.ok_or("No rasterizer specified.")?,
                vertex_buffers: self.vertex_buffers,
                attributes: self.attributes_desc,
                input_assembler: self
                    .input_assembler_desc
                    .ok_or("No input assembler desc specified.")?,
                blender: self.blender_desc.ok_or("No blender desc specified.")?,
                depth_stencil: self
                    .depth_stencil_desc
                    .ok_or("No depth stencil specified.")?,
                multisampling: None,
                baked_states: self.baked_states.ok_or("No states specified.")?,
                layout: &pipeline_layout,
                subpass: Subpass {
                    index: 0,
                    main_pass: self.render_pass,
                },
                flags: self.pipeline_creation_flags,
                parent: self.base_pipeline,
            };
            unsafe {
                self.device
                    .create_graphics_pipeline(&desc, None)
                    .map_err(|_| "Couldn't create a graphics pipeline!")?
            }
        };
        let device = self.device;
        for elem in self.shader_entries {
            unsafe { device.destroy_shader_module(elem.shader_module) }
        }
        Ok(Pipeline {
            descriptor_set,
            pipeline_layout,
            graphics_pipeline,
        })
    }
}

impl<'a, B, D> With<AttributeDesc> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: AttributeDesc) -> Self {
        self.attributes_desc.push(data);
        self
    }
}

impl<'a, B, D> With<DescriptorSetLayoutBinding> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: DescriptorSetLayoutBinding) -> Self {
        self.descriptor_set_layout_binding.push(data);
        self
    }
}

impl<'a, B, D> With<VertexBufferDesc> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: VertexBufferDesc) -> Self {
        self.vertex_buffers.push(data);
        self
    }
}

impl<'a, B, D> With<InputAssemblerDesc> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: InputAssemblerDesc) -> Self {
        self.input_assembler_desc = Some(data);
        self
    }
}

impl<'a, B, D> With<PipelineCreationFlags> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: PipelineCreationFlags) -> Self {
        self.pipeline_creation_flags = data;
        self
    }
}

impl<'a, B, D> With<Rasterizer> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: Rasterizer) -> Self {
        self.rasterizer = Some(data);
        self
    }
}

impl<'a, B, D> With<DepthStencilDesc> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: DepthStencilDesc) -> Self {
        self.depth_stencil_desc = Some(data);
        self
    }
}

impl<'a, B, D> With<BlendDesc> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: BlendDesc) -> Self {
        self.blender_desc = Some(data);
        self
    }
}

impl<'a, B, D> With<BakedStates> for PipelineBuilder<'a, B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    fn with(mut self, data: BakedStates) -> Self {
        self.baked_states = Some(data);
        self
    }
}
