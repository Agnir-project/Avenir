//! This example tries to create a simple example of a voxel world using rendy

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
    mesh::{Indices, Mesh, Model, PosColorNorm},
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    shader::{Shader, ShaderKind, ShaderSet, SourceLanguage, SourceShaderInfo, SpirvShader},
    wsi::Surface,
};

pub mod mesh;
pub mod scene;

use scene::*;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

// Shaders initialisation

// TODO: Spirv-reflection
use rendy::mesh::AsVertex;

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

    static ref SHADERS: rendy::shader::ShaderSetBuilder = rendy::shader::ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
struct UniformArgs {
    proj: nalgebra::Matrix4<f32>,
    view: nalgebra::Matrix4<f32>,
    lights_count: i32,
    pad: [i32; 3],
    lights: [Light; MAX_LIGHTS],
}

const MAX_LIGHTS: usize = 32;
const MAX_OBJECTS: usize = 1;
const UNIFORM_SIZE: u64 = std::mem::size_of::<UniformArgs>() as u64;
const MODELS_SIZE: u64 = std::mem::size_of::<Model>() as u64 * MAX_OBJECTS as u64;
const INDIRECT_SIZE: u64 = std::mem::size_of::<DrawIndexedCommand>() as u64;

// Utility functions

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

// Pipeline initialisation

#[derive(Debug, Default)]
struct VoxelRenderPipelineDesc;

#[derive(Debug)]
struct VoxelRenderPipeline<B: hal::Backend> {
    align: u64,
    buffer: Escape<Buffer<B>>,
    sets: Vec<Escape<DescriptorSet<B>>>,
}

impl<B> SimpleGraphicsPipelineDesc<B, Scene<B>> for VoxelRenderPipelineDesc
where
    B: hal::Backend,
{
    type Pipeline = VoxelRenderPipeline<B>;

    fn load_shader_set(&self, factory: &mut Factory<B>, _scene: &Scene<B>) -> ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        //TODO: Spirv-reflection
        vec![
            PosColorNorm::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex),
            Model::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Instance(1)),
        ]
    }

    fn layout(&self) -> Layout {
        Layout {
            sets: vec![SetLayout {
                bindings: vec![hal::pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: hal::pso::DescriptorType::UniformBuffer,
                    count: 1,
                    stage_flags: hal::pso::ShaderStageFlags::GRAPHICS,
                    immutable_samplers: false,
                }],
            }],
            push_constants: Vec::new(),
        }
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _scene: &Scene<B>,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Self::Pipeline, CreationError> {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert_eq!(set_layouts.len(), 1);

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
                Dynamic,
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

        Ok(VoxelRenderPipeline {
            align,
            buffer,
            sets,
        })
    }
}

impl<B> SimpleGraphicsPipeline<B, Scene<B>> for VoxelRenderPipeline<B>
where
    B: hal::Backend,
{
    type Desc = VoxelRenderPipelineDesc;

    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        index: usize,
        scene: &Scene<B>,
    ) -> PrepareResult {
        unsafe {
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    uniform_offset(index, self.align),
                    &[UniformArgs {
                        pad: [0, 0, 0],
                        proj: scene.camera.proj.to_homogeneous(),
                        view: scene.camera.view.inverse().to_homogeneous(),
                        lights_count: scene.lights.len() as i32,
                        lights: {
                            let mut array = [Light {
                                pad: 0.0,
                                pos: nalgebra::Vector3::new(0.0, 0.0, 0.0),
                                intensity: 0.0,
                            }; MAX_LIGHTS];
                            let count = std::cmp::min(scene.lights.len(), 32);
                            array[..count].copy_from_slice(&scene.lights[..count]);
                            array
                        },
                    }],
                )
                .unwrap()
        };

        unsafe {
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    indirect_offset(index, self.align),
                    &[DrawIndexedCommand {
                        index_count: scene.object_mesh.as_ref().unwrap().len(),
                        instance_count: scene.objects.len() as u32,
                        first_index: 0,
                        vertex_offset: 0,
                        first_instance: 0,
                    }],
                )
                .unwrap()
        };

        if !scene.objects.is_empty() {
            unsafe {
                factory
                    .upload_visible_buffer(
                        &mut self.buffer,
                        models_offset(index, self.align),
                        &scene.objects[..],
                    )
                    .unwrap()
            };
        }

        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        scene: &Scene<B>,
    ) {
        unsafe {
            encoder.bind_graphics_descriptor_sets(
                layout,
                0,
                Some(self.sets[index].raw()),
                std::iter::empty(),
            );

            //TODO: Spirv-reflection
            let vertex = [PosColorNorm::vertex()];

            scene
                .object_mesh
                .as_ref()
                .unwrap()
                .bind(0, &vertex, &mut encoder)
                .unwrap();
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

    fn dispose(self, _factory: &mut Factory<B>, _scene: &Scene<B>) {}
}

fn run<B: hal::Backend>(
    event_loop: EventLoop<()>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    _surface: Surface<B>,
    window: Window,
) {
    let mut graph_builder = GraphBuilder::<_, ()>::new();

    let size = window.inner_size();
    let window_kind = hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1);
    let _aspect = size.width / size.height;

    let depth = graph_builder.create_image(
        window_kind,
        1,
        hal::format::Format::D32Sfloat,
        Some(hal::command::ClearValue {
            depth_stencil: hal::command::ClearDepthStencil {
                depth: 1.0,
                stencil: 0,
            },
        }),
    );

    let _pass = graph_builder.add_node(
        crate::mesh::Pipeline::builder()
            .into_subpass()
            .with_depth_stencil(depth)
            .into_pass(),
    );

    let graph = graph_builder
        .build(&mut factory, &mut families, &())
        .unwrap();

    let mut frame = 0u64;
    let mut graph = Some(graph);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => {
                    println!("{:#?}", input);
                    if let Some(code) = input.virtual_keycode {
                        match code {
                            VirtualKeyCode::D => {}
                            VirtualKeyCode::Z => {}
                            VirtualKeyCode::Q => {}
                            VirtualKeyCode::S => {}
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                factory.maintain(&mut families);
                if let Some(ref mut graph) = graph {
                    graph.run(&mut factory, &mut families, &());
                    frame += 1;
                }
            }
            _ => {}
        }
        if *control_flow == ControlFlow::Exit {
            if let Some(graph) = graph.take() {
                graph.dispose(&mut factory, &());
            }
        }
    });
}

fn main() {
    let config: Config = Default::default();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(960, 640))
        .with_title("Rendy test");

    let rendy = AnyWindowedRendy::init(
        EnabledBackend::which::<Backend>(),
        &config,
        window,
        &event_loop,
    )
    .unwrap();
    rendy::with_any_windowed_rendy!((rendy)
        use back;
        (factory, families, surface, window) => {

        run(event_loop, factory, families, surface, window);
    });
}
