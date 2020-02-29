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
    wsi::Surface,
};

pub mod mesh;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(not(any(feature = "metal", feature = "vulkan", feature = "dx12")))]
type Backend = rendy::empty::Backend;

// Shaders initialisation

const MAX_LIGHTS: usize = 32;
const MAX_OBJECTS: usize = 1;
const UNIFORM_SIZE: u64 = std::mem::size_of::<crate::mesh::UniformArgs>() as u64;
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
                float32: [1., 0.5, 1., 0.],
            },
        }),
    );

    let meshpass = graph_builder.add_node(
        crate::mesh::Pipeline::builder()
            .into_subpass()
            .with_depth_stencil(depth)
            .with_color(color)
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
                    if let Some(code) = input.virtual_keycode {
                        match code {
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
        (factory, families, surface, window) => { run(event_loop, factory, families, surface, window) });
}
