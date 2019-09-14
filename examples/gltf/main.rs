use avenir;
use gltf::{Glb, Gltf};
use std::{fs, io, path::Path};
use winit::{dpi::LogicalSize, Event, EventsLoop, Window, WindowBuilder, WindowEvent};

fn main() {
    let (document, buffers, data) = gltf::import("./examples/gltf/BoomBox.glb").unwrap();

    for mesh in document.meshes() {
        println!("Mesh #{}", mesh.index());
        for primitive in mesh.primitives() {
            println!("- Primitive #{}", primitive.index());
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            if let Some(iter) = reader.read_positions() {
                for vertex_position in iter {
                    println!("{:?}", vertex_position);
                }
            }
        }
    }

    let evt_loop = EventsLoop::new();
    let window = WindowBuilder::new()
        .with_dimensions(LogicalSize::new(1600.0, 900.0))
        .with_title("Hello gltf")
        .build(&evt_loop)
        .expect("Cannot create window.");

    render_loop(window, evt_loop);
}

fn render_loop(window: Window, mut evt_loop: EventsLoop) {
    let mut is_open = false;

    while is_open {
        evt_loop.poll_events(|e| handle_event(e, &mut is_open));
    }
}

fn handle_event(e: Event, is_open: &mut bool) {
    match e {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *is_open = false,
            _ => (),
        },
        _ => (),
    }
}
