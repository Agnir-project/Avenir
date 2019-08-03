#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
extern crate log;
extern crate render_lib;
extern crate simple_logger;
extern crate winit;


pub const VERTEX_SOURCE: &str = "#version 450
layout (location = 0) in vec2 position;
layout (location = 1) in vec3 color;

layout (location = 0) out gl_PerVertex {
  vec4 gl_Position;
};
layout (location = 1) out vec3 frag_color;

void main()
{
  gl_Position = vec4(position, 0.0, 1.0);
  frag_color = color;
}";

pub const FRAGMENT_SOURCE: &str = "#version 450
layout (location = 1) in vec3 frag_color;

layout (location = 0) out vec4 color;

void main()
{
  color = vec4(frag_color,1.0);
}";


use gfx_hal::{
    window::CompositeAlpha::{Inherit, Opaque, PostMultiplied, PreMultiplied},
    window::PresentMode::{Fifo, Immediate, Mailbox, Relaxed},
};

use render_lib::{hal_state::HalState, hal_state::HalStateOptions, Triangle};
use winit::{
    dpi::LogicalSize, CreationError, Event, EventsLoop, Window, WindowBuilder, WindowEvent,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalState {
    pub frame_width: f64,
    pub frame_height: f64,
    pub mouse_x: f64,
    pub mouse_y: f64,
}

impl LocalState {
    pub fn update_from_input(&mut self, input: UserInput) {
        if let Some(frame_size) = input.new_frame_size {
            self.frame_width = frame_size.0;
            self.frame_height = frame_size.1;
        }
        if let Some(position) = input.new_mouse_position {
            self.mouse_x = position.0;
            self.mouse_y = position.1;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserInput {
    pub end_requested: bool,
    pub new_frame_size: Option<(f64, f64)>,
    pub new_mouse_position: Option<(f64, f64)>,
}

impl UserInput {
    pub fn poll_events_loop(events_loop: &mut EventsLoop) -> Self {
        let mut output = UserInput::default();
        events_loop.poll_events(|event| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => output.end_requested = true,
            Event::WindowEvent {
                event: WindowEvent::Resized(logical),
                ..
            } => {
                output.new_frame_size = Some((logical.width, logical.height));
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                output.new_mouse_position = Some((position.x, position.y));
            }
            _ => (),
        });
        output
    }
}

#[derive(Debug)]
pub struct WinitState {
    pub events_loop: EventsLoop,
    pub window: Window,
}

impl WinitState {
    pub fn new<T: Into<String>>(title: T, size: LogicalSize) -> Result<Self, CreationError> {
        let events_loop = EventsLoop::new();
        let output = WindowBuilder::new()
            .with_title(title)
            .with_dimensions(size)
            .build(&events_loop);
        output.map(|window| Self {
            events_loop,
            window,
        })
    }
}

impl Default for WinitState {
    fn default() -> Self {
        Self::new(
            "Main Window",
            LogicalSize {
                width: 800.0,
                height: 600.0,
            },
        )
        .expect("Could not create a window!")
    }
}

fn do_the_render(hal_state: &mut HalState, local_state: &LocalState) -> Result<(), &'static str> {
    let x = ((local_state.mouse_x / local_state.frame_width) * 2.0) - 1.0;
    let y = ((local_state.mouse_y / local_state.frame_height) * 2.0) - 1.0;
    let triangle1 = Triangle {
        points: [[-0.5, 0.5], [-0.5, -0.5], [x as f32, y as f32]],
    };
    let triangle2 = Triangle {
        points: [[-0.5, 0.5], [0.5, 0.5], [x as f32, y as f32]],
    };
    hal_state.draw_triangle_frame(triangle1)
}

fn main() {
    simple_logger::init().unwrap();

    let options = HalStateOptions {
        pm_order: vec![Mailbox, Fifo, Relaxed, Immediate],
        ca_order: vec![Opaque, Inherit, PreMultiplied, PostMultiplied],
        shaders: &[
            (shaderc::ShaderKind::Vertex, VERTEX_SOURCE.to_string()),
            (shaderc::ShaderKind::Fragment, FRAGMENT_SOURCE.to_string()),
        ]
    };
    let mut winit_state = WinitState::default();

    let mut hal_state = match HalState::new(&winit_state.window, &options) {
        Ok(state) => state,
        Err(e) => panic!(e),
    };
    let (frame_width, frame_height) = winit_state
        .window
        .get_inner_size()
        .map(|logical| logical.into())
        .unwrap_or((0.0, 0.0));
    let mut local_state = LocalState {
        frame_width,
        frame_height,
        mouse_x: 0.0,
        mouse_y: 0.0,
    };

    loop {
        let inputs = UserInput::poll_events_loop(&mut winit_state.events_loop);
        if inputs.end_requested {
            break;
        }
        local_state.update_from_input(inputs);
        if let Err(e) = do_the_render(&mut hal_state, &local_state) {
            error!("Rendering Error: {:?}", e);
            debug!("Auto-restarting HalState...");
            drop(hal_state);
            hal_state = match HalState::new(&winit_state.window, &options) {
                Ok(state) => state,
                Err(e) => panic!(e),
            };
        }
    }
}
