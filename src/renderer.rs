//
//  Rust file | 2018
//  Author: Alexandre Fourcat
//  renderer.rs
//  module:
//! High level api for the user.

use crate::hal_state;

#[derive(Default, Debug)]
struct RendererBuilder {
    render_color: Option<Color>,
    window_size: Option<(f32, f32)>,
}

#[derive(Default, Debug)]
struct Renderer {
    render_context: Context,
    clear_color: Color,
}

impl RendererBuilder {
    fn build(self) -> Renderer {
        Renderer {
            render_context: RenderContext::new(),
            clear_color: self.render_color.unwrap_or(Color::new(0.0, 0.0, 0.0)),
        }
    }
    
    fn with_window_size(self, width: f32, height: f32) -> Self {
        self.window_size = (width, height);
        self
    }

    fn with_memory_layout() -> Self {
        self
    }
}

impl Renderer {
    fn builder() -> RendererBuilder {
        RendererBuilder::default()
    }
}
