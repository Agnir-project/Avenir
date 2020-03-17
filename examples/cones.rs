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
            event::{
                DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState, VirtualKeyCode,
                WindowEvent,
            },
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

use avenir::{camera::Camera, graph, Inputs};
use env_logger;
use nalgebra::Point3;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "empty")]
type Backend = rendy::empty::Backend;

const WIDTH: u32 = 3840;
const HEIGHT: u32 = 2160;

#[allow(dead_code)] // Bug in rust-analyzer.
fn run<B: hal::Backend>(
    event_loop: EventLoop<()>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    surface: Surface<B>,
    window: Window,
) {
    let mut frame = 0u64;
    let mut cam = Camera::look_at(
        10.0,
        Point3::new(0.0, 0.0, -10.0),
        Point3::new(0.0, 0.0, 0.0),
        WIDTH as f32 / HEIGHT as f32,
    );
    let mut inputs: Inputs = Inputs::default();
    let mut graph =
        Some(graph::build(&mut families, &window, &mut factory, surface, &cam).unwrap());

    let started = std::time::Instant::now();
    let mut checkpoint = started;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::DeviceEvent { ref event, .. } => match *event {
                DeviceEvent::MouseMotion { delta: (x, y) } => {
                    inputs.mouse_x = x;
                    inputs.mouse_y = y;
                }
                _ => {}
            },
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    info!("Window Resized {:?}.", size);
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state,
                            ..
                        },
                    ..
                } => match (virtual_code, state) {
                    (VirtualKeyCode::A, ElementState::Pressed) => inputs.left = true,
                    (VirtualKeyCode::A, ElementState::Released) => inputs.left = false,
                    (VirtualKeyCode::S, ElementState::Pressed) => inputs.back = true,
                    (VirtualKeyCode::S, ElementState::Released) => inputs.back = false,
                    (VirtualKeyCode::D, ElementState::Pressed) => inputs.right = true,
                    (VirtualKeyCode::D, ElementState::Released) => inputs.right = false,
                    (VirtualKeyCode::W, ElementState::Pressed) => inputs.front = true,
                    (VirtualKeyCode::W, ElementState::Released) => inputs.front = false,
                    (VirtualKeyCode::L, ElementState::Pressed) => cam.ambient_power += 0.1,
                    (VirtualKeyCode::K, ElementState::Pressed) => cam.ambient_power -= 0.1,
                    _ => {}
                },
                _ => {}
            },
            Event::MainEventsCleared => {
                factory.maintain(&mut families);
                if let Some(ref mut graph) = graph {
                    graph.run(&mut factory, &mut families, &cam);
                    frame += 1;
                }
                let elapsed = checkpoint.elapsed();
                // Print fps
                // let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;
                // info!("FPS: {} delta: {}", frame * 1_000_000_000 / elapsed_ns, elapsed.as_secs_f32());
                frame = 0;
                checkpoint += elapsed;
                cam.run(&inputs, elapsed.as_secs_f32());
                inputs.mouse_x = 0.0;
                inputs.mouse_y = 0.0;
            }
            Event::RedrawRequested(_) => {
                if let Some(ref mut graph) = graph {
                    graph.run(&mut factory, &mut families, &cam);
                    frame += 1;
                }

                info!("Request redraw.");
            }
            _ => {}
        }
        if *control_flow == ControlFlow::Exit {
            if let Some(graph) = graph.take() {
                graph.dispose(&mut factory, &cam);
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

    let rendy = AnyWindowedRendy::init_auto(&config, window, &event_loop).unwrap();
    rendy::with_any_windowed_rendy!((rendy)
    use back;
    (factory, families, surface, window) => {
        window.set_cursor_grab(true);
        run(event_loop, factory, families, surface, window)
    });
}
