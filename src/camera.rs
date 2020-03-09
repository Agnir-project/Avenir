use nalgebra::{Isometry3, Perspective3, Vector3, Translation3};
use crate::mesh::UniformArgs;
use crate::Inputs;

pub struct Camera {
    pub view: Isometry3<f32>,
    pub proj: Perspective3<f32>,
    pub rotation_speed: f32,
}

impl Camera {
    pub fn look_at(
        eye: nalgebra::Point3<f32>,
        target: nalgebra::Point3<f32>,
        aspect: f32,
    ) -> Self {
        let view = nalgebra::Isometry3::look_at_rh(&eye, &target, &Vector3::y());
        let proj = Perspective3::new(aspect, std::f32::consts::FRAC_PI_3, 1.0, 200.0);

        Camera { proj, view, rotation_speed: 0.01 }
    }

    pub fn center_euler(&mut self, inputs: Inputs) {
        let axis_angle = if inputs.right_rot {
            Vector3::y() * self.rotation_speed
        } else if inputs.down_rot {
            Vector3::x() * self.rotation_speed
        } else if inputs.left_rot {
            Vector3::y() * -self.rotation_speed
        } else if inputs.up_rot {
            Vector3::x() * -self.rotation_speed
        } else {
            Vector3::x() * 0.0
        };
        let rot = nalgebra::UnitQuaternion::new(axis_angle);
        self.view.append_rotation_wrt_center_mut(&rot);
    }

    pub fn translate(&mut self, translation: &Translation3<f32>) {
        self.view.append_translation_mut(translation);
    }
}

impl Into<UniformArgs> for Camera {
    fn into(self) -> UniformArgs {
        UniformArgs {
            view: self.view.inverse().to_homogeneous(),
            proj: self.proj.to_homogeneous(),
        }
    }
}
