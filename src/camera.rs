use nalgebra::{Isometry3, Perspective3, UnitQuaternion, Vector3};
use crate::Inputs;

/// Represent a configurable camera in 3D.
pub struct Camera {
    /// The movement speed of the camera along axis.
    pub speed: f32,

    /// The rotation sensitivity, often linked to mouse movement.
    pub sensitivity: f64,

    /// View matrix, represent Camera position and rotation.
    pub view: Isometry3<f32>,

    /// Projection matrix, transform 3D world to 2D coordinate.
    pub proj: Perspective3<f32>,

    /// Test TODO: Remove
    pub ambient_power: f32,
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
            sensitivity: 0.01,
            view: nalgebra::Isometry3::look_at_rh(&eye, &target, &Vector3::y()),
            proj: Perspective3::new(aspect, std::f32::consts::FRAC_PI_3, 1.0, 400.0),
            ambient_power: 1.0,
        }
    }

    /// Provide input to update camera. TODO: Decouple inputs and Camera.
    pub fn run(&mut self, inputs: &Inputs, delta_sec: f32) {
        let x = if inputs.right && inputs.left {
            0.0
        } else if inputs.right {
            1.0
        } else if inputs.left {
            -1.0
        } else {
            0.0
        };
        let y = if inputs.up && inputs.down {
            0.0
        } else if inputs.up {
            1.0
        } else if inputs.down {
            -1.0
        } else {
            0.0
        };
        let z = if inputs.front && inputs.back {
            0.0
        } else if inputs.front {
            -1.0
        } else if inputs.back {
            1.0
        } else {
            0.0
        };

        self.view.rotation *= UnitQuaternion::from_axis_angle(
            &Vector3::x_axis(),
            (inputs.mouse_y * self.sensitivity) as f32,
        );

        let q = UnitQuaternion::from_axis_angle(
            &Vector3::y_axis(),
            (-inputs.mouse_x * self.sensitivity) as f32,
        );
        self.view.rotation = q * self.view.rotation;

        let translation = Vector3::new(x, y, z);

        let rotation_translation =
            self.view.rotation * translation * (delta_sec as f32 * self.speed);
        self.view.translation.vector += rotation_translation;
    }
}
