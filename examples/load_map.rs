//
//  Rust file | 2018
//  Author: Alexandre Fourcat
//  load_map.rs
//  module:
//! Trying to load a voxel map with avenir.

use avenir::prelude::{Camera, Renderer, Color};

use std::path::PathBuf;

const speed: f32 = 100.0;

fn main() {
    // Renderer parameters.
    let rendere_config = RendererParams {
        window_size: (1600, 900),
        window_type: WindowType::Windowed,
        window_name: "Map example".to_string(),
        assets_folder: PathBuf::from("./assets/"),
        memory_layout: Default::default(),
        clear_color: Color(0.0, 0.0, 0.0),
    };

    // Initializing renderer.
    let (window, renderer) = Renderer::new(parameters);

    // Loading map configuration from default.
    let map_config = MapConfig::default();

    // Creating map from config.
    let map = Map::new(map_config);

    // Creating camera.
    let camera: Camera = renderer.create_camera(0.0, 1.0, 0.0);

    // Game loop.
    while window.is_open() {
        for evt in window.poll_events() {
            handle_event(event, &mut camera);
        }

        render_map(&mut renderer, &map, &camera);
    }
}

fn render_map(renderer: &mut Renderer, map: &Map, camera: &mut Camera) {
    // Iterating through chunks.
    for chunk in map.chunks() {
        /// Getting chunk position.
        let pos = chunk.position();

        /// Getting updated mesh informations.
        let mesh = if chunk.need_mesh_update() {
            chunk.generate_mesh();
        } else {
            chunk.get_mesh();
        };

        // Render static mesh (default).
        if let Err(e) = renderer.render_mesh(pos, mesh, camera) {
            println!("Error while rendering mesh {}", e);
        }
    }
}

fn handle_event(event: Event, camera: &mut Camera) {
    match event {
        Event::Window(window_event) => match window_event {
            WindowEvent::Close => window.should_close(),
            _ => (),
        },
        Event::Keyboard(input_event) => match input_event {
            Keyboard::W => camera.translate((0.0, 0.0, speed)),
            Keyboard::A => camera.translate((0.0, 0.0, speed)),
            Keyboard::D => camera.translate((0.0, 0.0, speed)),
            Keyboard::S => camera.translate((0.0, 0.0, speed)),
            _ => (),
        },
    }
}
