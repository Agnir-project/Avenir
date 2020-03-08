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

#[macro_use]
extern crate log;

use env_logger;

pub mod graph;
pub mod mesh;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "empty")]
type Backend = rendy::empty::Backend;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

// Shaders initialisation

fn run<B: hal::Backend>(
    event_loop: EventLoop<()>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    surface: Surface<B>,
    window: Window,
) {
    let mut frame = 0u64;
    let mut graph = Some(graph::build(&mut families, &window, &mut factory, surface).unwrap());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    info!("Window Resized {:?}.", size);
                }
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
    env_logger::init();
    info!("Starting Avenir");

    let config: Config = Default::default();
    let event_loop = EventLoop::new();

    info!("Creating Window of {} by {} pixels.", WIDTH, HEIGHT);
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
        .with_title("Avenir");

    let rendy = AnyWindowedRendy::init_auto(&config, window, &event_loop,).unwrap();
    rendy::with_any_windowed_rendy!((rendy)
        use back;
        (factory, families, surface, window) => {
            run(event_loop, factory, families, surface, window)
        });
}
