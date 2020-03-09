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
            event::{Event, VirtualKeyCode, WindowEvent, ElementState, KeyboardInput},
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

use camera::Camera;
use env_logger;
use nalgebra::{Point3, Vector3};

pub mod graph;
pub mod camera;
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

#[derive(Default, Copy, Clone)]
pub struct Inputs {
    left_rot: bool,
    right_rot: bool,
    up_rot: bool,
    down_rot: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

fn get_translation(speed: f32, inputs: Inputs) -> nalgebra::Translation3<f32> {
    let mut t = nalgebra::Translation3::identity();

    if inputs.up {
        t *= nalgebra::Translation3::from(nalgebra::Vector3::new(0.0, 0.0, speed));
    }
    if inputs.left {
        t *= nalgebra::Translation3::from(nalgebra::Vector3::new(-speed, 0.0, 0.0));
    }
    if inputs.down {
        t *= nalgebra::Translation3::from(nalgebra::Vector3::new(0.0, 0.0, -speed));
    }
    if inputs.right {
        t *= nalgebra::Translation3::from(nalgebra::Vector3::new(speed, 0.0, 0.0));
    }
    t
}

// Shaders initialisation

fn run<B: hal::Backend>(
    event_loop: EventLoop<()>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    surface: Surface<B>,
    window: Window,
) {
    let mut frame = 0u64;
    let mut cam = Camera::look_at(
        Point3::new(0.0, 0.0, -10.0),
        Point3::new(0.0, 0.0, 0.0),
        WIDTH as f32 / HEIGHT as f32,
    );
    let mut inputs: Inputs = Inputs::default();
    let mut graph =
        Some(graph::build(&mut families, &window, &mut factory, surface, &cam).unwrap());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        let translation = get_translation(0.2, inputs);
        cam.translate(&translation);
        cam.center_euler(inputs);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    info!("Window Resized {:?}.", size);
                },
                WindowEvent::KeyboardInput {
                  input : KeyboardInput {
                      virtual_keycode: Some(virtual_code),
                      state,
                      ..
                    },
                  ..
                } => match (virtual_code, state) {
                  (VirtualKeyCode::A, ElementState::Pressed) => inputs.left_rot = true,
                  (VirtualKeyCode::A, ElementState::Released) => inputs.left_rot = false,
                  (VirtualKeyCode::S, ElementState::Pressed) => inputs.down_rot = true,
                  (VirtualKeyCode::S, ElementState::Released) => inputs.down_rot = false,
                  (VirtualKeyCode::D, ElementState::Pressed) => inputs.right_rot = true,
                  (VirtualKeyCode::D, ElementState::Released) => inputs.right_rot = false,
                  (VirtualKeyCode::W, ElementState::Pressed) => inputs.up_rot = true,
                  (VirtualKeyCode::W, ElementState::Released) => inputs.up_rot = false,

                  (VirtualKeyCode::Up, ElementState::Pressed) => inputs.up = true,
                  (VirtualKeyCode::Up, ElementState::Released) => inputs.up = false,
                  (VirtualKeyCode::Down, ElementState::Pressed) => inputs.down = true,
                  (VirtualKeyCode::Down, ElementState::Released) => inputs.down = false,
                  (VirtualKeyCode::Left, ElementState::Pressed) => inputs.left = true,
                  (VirtualKeyCode::Left, ElementState::Released) => inputs.left = false,
                  (VirtualKeyCode::Right, ElementState::Pressed) => inputs.right = true,
                  (VirtualKeyCode::Right, ElementState::Released) => inputs.right = false,
                  _ => {}
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                factory.maintain(&mut families);

                if let Some(ref mut graph) = graph {
                    graph.run(&mut factory, &mut families, &cam);
                    frame += 1;
                }
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
        run(event_loop, factory, families, surface, window)
    });
}
