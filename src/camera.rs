use crate::mesh::UniformArgs;
use crate::Inputs;
use nalgebra::{Isometry3, Perspective3, Translation3, Vector3, Unit, UnitQuaternion};

pub struct Camera {
    pub speed: f32,
    pub sensitivity: f64,
    pub view: Isometry3<f32>,
    pub proj: Perspective3<f32>,
}

impl Camera {
    /// Builds a new `FlyMovementSystem` using the provided speeds and axis controls.
    pub fn look_at(
        speed: f32,
        eye: nalgebra::Point3<f32>,
        target: nalgebra::Point3<f32>,
        aspect: f32,
    ) -> Self {
        Camera {
            speed,
            sensitivity: 0.001,
            view: nalgebra::Isometry3::look_at_rh(&eye, &target, &Vector3::y()),
            proj: Perspective3::new(aspect, std::f32::consts::FRAC_PI_3, 1.0, 400.0),
        }
    }

    pub fn run(&mut self, inputs: &Inputs, delta_sec: f32) {
        let x = if inputs.right && inputs.left { 0.0 } else if inputs.right { 1.0 } else if inputs.left { -1.0 } else { 0.0 };
        let y = if inputs.up && inputs.down { 0.0 } else if inputs.up { 1.0 } else if inputs.down { -1.0 } else { 0.0 };
        let z = if inputs.front && inputs.back { 0.0 } else if inputs.front { 1.0 } else if inputs.back { -1.0 } else { 0.0 };

        self.view.rotation *= UnitQuaternion::from_axis_angle(&Vector3::x_axis(), (-inputs.mouse_y * self.sensitivity) as f32);

        let q = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), (-inputs.mouse_x * self.sensitivity) as f32);
        self.view.rotation = q * self.view.rotation;

        if let Some(dir) = Unit::try_new(Vector3::new(x, y, z), nalgebra::convert(1.0e-6)) {
            self.view.translation.vector += self.view.rotation * dir.as_ref() * (delta_sec as f32 * self.speed);
        }
    }
}