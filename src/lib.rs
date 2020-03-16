/// Avenir
/// Voxel rendering crate early stage.

pub mod camera;
pub mod mesh;
pub mod graph;

#[macro_use]
extern crate log;

#[derive(Default, Copy, Clone, Debug)]
/// Temporary struct representing user's inputs.
pub struct Inputs {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub front: bool,
    pub back: bool,
    pub mouse_x: f64,
    pub mouse_y: f64,
}
