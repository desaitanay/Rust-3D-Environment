/// Represent the camera in the screen.

/// Represents the camera in easier user friendly format
pub struct Camera {
    /// Location of camera
    pub eye: cgmath::Point3<f32>,
    /// Direction camera is pointing
    pub target: cgmath::Point3<f32>,
    /// The direction of up
    pub up: cgmath::Vector3<f32>,
    /// aspect ratio for camera
    pub aspect: f32,
    /// field of view in degrees
    pub fovy: f32,
    /// distance to the near clipping plane
    pub znear: f32,
    /// distance to the far clipping plane
    pub zfar: f32,
}

// translation matrix that translates from openGL space to wGPU space
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

impl Camera {
    /// Convert the user friendly camera information to one camera matrix
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // matrix to represent the location of the camera
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // matrix to represent the depth/perspective of the camera
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
/// Represents the camera information as one matrix
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    /// Create new camera matrix object
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    /// Update the camera matrix based off the camera values
    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

#[cfg(test)]
mod tests {
    use crate::state::camera;

    #[test]
    fn test_update_view_proj() {
        let fake_camera = camera::Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: 1.0,
            fovy: 90.0,
            znear: 1.0,
            zfar: 100.0,
        };

        let mut camera_uniform = camera::CameraUniform::new();

        camera_uniform.update_view_proj(&fake_camera);
        assert_eq!(fake_camera.build_view_projection_matrix(), camera_uniform.view_proj.into());
    }

    #[test]
    fn test_build_view_projection_matrix() {
        let fake_camera = camera::Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: 1.0,
            fovy: 90.0,
            znear: 1.0,
            zfar: 100.0,
        };

        let return_value = fake_camera.build_view_projection_matrix();
        let expected=[
            [1.0,0.00000000,0.00000000, 0.00000000],
            [0.00000000, 0.894427180, -0.228124112, -0.675337732],
            [0.00000000, -0.447213590, -0.456248224, -1.35067546],
            [0.00000000, 0.00000000, 0.130519629, 2.36658764]
        ];

        assert_eq!(return_value, expected.into());

    }

}