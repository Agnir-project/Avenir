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
        present::PresentNode,
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

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

// Shaders initialisation

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
    model: nalgebra::Matrix4<f32>,
    proj: nalgebra::Matrix4<f32>,
    view: nalgebra::Matrix4<f32>,
}

const MAX_LIGHTS: usize = 32;
const MAX_OBJECTS: usize = 1;
const UNIFORM_SIZE: u64 = std::mem::size_of::<UniformArgs>() as u64;
const MODELS_SIZE: u64 = std::mem::size_of::<Model>() as u64 * MAX_OBJECTS as u64;
const INDIRECT_SIZE: u64 = std::mem::size_of::<DrawIndexedCommand>() as u64;

fn run<B: hal::Backend>(
    event_loop: EventLoop<()>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    surface: Surface<B>,
    window: Window,
) {
    let mut graph_builder = GraphBuilder::<B, ()>::new();

    let size = window.inner_size();

    let window_kind = hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1);
    let _aspect = size.width / size.height;

    // Create the depth stencil image.
    let depth = graph_builder.create_image(
        window_kind,
        1,
        hal::format::Format::D32Sfloat,
        Some(hal::command::ClearValue {
            depth_stencil: hal::command::ClearDepthStencil {
                depth: 1.,
                stencil: 0,
            },
        }),
    );

    let color = graph_builder.create_image(
        window_kind,
        1,
        factory.get_surface_format(&surface),
        Some(hal::command::ClearValue {
            color: hal::command::ClearColor {
                float32: [1., 0., 1., 0.],
            },
        }),
    );

    let meshpass = graph_builder.add_node(
        crate::mesh::Pipeline::builder()
            .into_subpass()
            .with_depth_stencil(depth)
            .into_pass(),
    );

    graph_builder
        .add_node(PresentNode::builder(&factory, surface, color).with_dependency(meshpass));

    let graph = graph_builder
                    .with_frames_in_flight(3)
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
        .with_inner_size(LogicalSize::new(1920, 1080))
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
