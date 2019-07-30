#![feature(type_ascription)]
extern crate shaderc;
extern crate winit;

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub points: [[f32; 2]; 3],
}

impl Triangle {
    pub fn points_flat(self) -> [f32; 6] {
        let [[a, b], [c, d], [e, f]] = self.points;
        [a, b, c, d, e, f]
    }
    pub fn vertex_attributes(self) -> [f32; 3 * (2 + 3)] {
        let [[a, b], [c, d], [e, f]] = self.points;
        [
            a, b, 1.0, 0.0, 0.0, // red
            c, d, 0.0, 1.0, 0.0, // green
            e, f, 0.0, 0.0, 1.0, // blue
        ]
    }
}

#[cfg(feature = "dx12")]
pub use gfx_backend_dx12 as back;

#[cfg(feature = "metal")]
pub use gfx_backend_metal as back;

#[cfg(feature = "vulkan")]
pub use gfx_backend_vulkan as back;

pub mod buffer_bundle;
pub mod gfx_utils;
pub mod hal_state;
pub mod pipeline;
pub mod shader_utils;
mod utils;
