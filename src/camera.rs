//! Camera module - handles perspective camera and orbit controls

use glam::{Mat4, Vec3};

/// Orbital camera with perspective projection
pub struct Camera {
    /// Camera distance from origin
    pub distance: f32,
    /// Horizontal angle in degrees
    pub angle_y: f32,
    /// Vertical angle in degrees  
    pub angle_x: f32,
    /// Field of view in degrees
    pub fov: f32,
    /// Aspect ratio (width / height)
    pub aspect: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Look-at target
    pub target: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            distance: 3.5,
            angle_y: 45.0,
            angle_x: 45.0,
            fov: 45.0,
            aspect: 16.0 / 9.0,
            near: 0.01,
            far: 100.0,
            target: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Camera {
    /// Calculate camera position based on orbital angles
    pub fn position(&self) -> Vec3 {
        let rad_x = self.angle_x.to_radians();
        let rad_y = self.angle_y.to_radians();
        
        Vec3::new(
            rad_y.sin() * self.distance * rad_x.cos(),
            rad_x.sin() * self.distance,
            rad_y.cos() * self.distance * rad_x.cos(),
        )
    }
    
    /// Get view matrix
    pub fn view_matrix(&self) -> Mat4 {
        let pos = self.position();
        Mat4::look_at_rh(pos, self.target, Vec3::Y)
    }
    
    /// Get projection matrix
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov.to_radians(),
            self.aspect,
            self.near,
            self.far,
        )
    }
    
    /// Get combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
    
    /// Update aspect ratio
    pub fn set_aspect(&mut self, width: f32, height: f32) {
        self.aspect = width / height;
    }
    
    /// Orbit the camera (from mouse drag)
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.angle_y -= delta_x;
        self.angle_x -= delta_y;
        // Clamp vertical angle to avoid gimbal lock
        self.angle_x = self.angle_x.clamp(-89.999, 89.999);
    }
    
    /// Calculate visible area at water level (y=0)
    pub fn visible_area(&self) -> (f32, f32) {
        let visible_height = 2.0 * self.distance * (self.fov.to_radians() / 2.0).tan();
        let visible_width = visible_height * self.aspect;
        (visible_width, visible_height)
    }
}

/// Camera uniform data for shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub eye: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            view: Mat4::IDENTITY.to_cols_array_2d(),
            proj: Mat4::IDENTITY.to_cols_array_2d(),
            eye: [0.0, 0.0, 0.0, 1.0],
        }
    }
    
    pub fn update(&mut self, camera: &Camera) {
        self.view_proj = camera.view_projection_matrix().to_cols_array_2d();
        self.view = camera.view_matrix().to_cols_array_2d();
        self.proj = camera.projection_matrix().to_cols_array_2d();
        let pos = camera.position();
        self.eye = [pos.x, pos.y, pos.z, 1.0];
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new()
    }
}
