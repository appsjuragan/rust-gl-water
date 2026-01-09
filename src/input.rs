//! Input module - handles mouse/keyboard input and raycasting

use glam::{Mat4, Vec2, Vec3, Vec4};

/// Interaction modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionMode {
    None,
    AddDrops,
    MoveSphere,
    OrbitCamera,
}

/// Input manager for handling user interactions
pub struct InputManager {
    /// Current interaction mode
    pub mode: InteractionMode,
    /// Current mouse position in screen coords
    pub mouse_pos: Vec2,
    /// Previous mouse position
    pub prev_mouse_pos: Vec2,
    /// Mouse point in world space (on water plane)
    pub mouse_point: Option<Vec3>,
    /// Whether mouse is pressed
    pub mouse_pressed: bool,
    /// Previous hit point for dragging
    prev_hit: Vec3,
    /// Plane normal for sphere dragging
    plane_normal: Vec3,
    /// Screen size
    pub screen_size: Vec2,
}

impl Default for InputManager {
    fn default() -> Self {
        Self {
            mode: InteractionMode::None,
            mouse_pos: Vec2::ZERO,
            prev_mouse_pos: Vec2::ZERO,
            mouse_point: None,
            mouse_pressed: false,
            prev_hit: Vec3::ZERO,
            plane_normal: Vec3::Y,
            screen_size: Vec2::new(800.0, 600.0),
        }
    }
}

impl InputManager {
    /// Handle mouse move event
    pub fn on_mouse_move(&mut self, x: f32, y: f32, view_proj_inv: Mat4) {
        self.prev_mouse_pos = self.mouse_pos;
        self.mouse_pos = Vec2::new(x, y);
        
        // Update world space mouse point
        let ndc = self.screen_to_ndc(x, y);
        self.mouse_point = self.raycast_water_plane(ndc, view_proj_inv);
    }
    
    /// Handle mouse button press
    pub fn on_mouse_down(
        &mut self,
        x: f32,
        y: f32,
        view_proj_inv: Mat4,
        view_dir: Vec3,
        sphere_center: Vec3,
        sphere_radius: f32,
        pool_width: f32,
        pool_length: f32,
    ) {
        self.mouse_pressed = true;
        self.mouse_pos = Vec2::new(x, y);
        self.prev_mouse_pos = self.mouse_pos;
        
        let ndc = self.screen_to_ndc(x, y);
        let ray = self.get_ray(ndc, view_proj_inv);
        
        // 1. Hit test sphere
        if let Some(hit) = self.ray_sphere_intersection(ray.0, ray.1, sphere_center, sphere_radius) {
            self.mode = InteractionMode::MoveSphere;
            self.prev_hit = hit;
            self.plane_normal = -view_dir;
        }
        // 2. Hit test water plane
        else if let Some(point) = self.raycast_water_plane(ndc, view_proj_inv) {
            let half_w = pool_width / 2.0;
            let half_l = pool_length / 2.0;
            
            if point.x.abs() < half_w && point.z.abs() < half_l {
                self.mode = InteractionMode::AddDrops;
            } else {
                self.mode = InteractionMode::OrbitCamera;
            }
        } else {
            self.mode = InteractionMode::OrbitCamera;
        }
    }
    
    /// Handle mouse button release
    pub fn on_mouse_up(&mut self) {
        self.mouse_pressed = false;
        self.mode = InteractionMode::None;
    }
    
    /// Get mouse delta for camera orbit
    pub fn get_orbit_delta(&self) -> Vec2 {
        self.mouse_pos - self.prev_mouse_pos
    }
    
    /// Get drop position if in AddDrops mode
    pub fn get_drop_position(&self) -> Option<Vec3> {
        if self.mode == InteractionMode::AddDrops {
            self.mouse_point
        } else {
            None
        }
    }
    
    /// Get sphere drag delta
    pub fn get_sphere_drag_delta(&mut self, view_proj_inv: Mat4) -> Option<Vec3> {
        if self.mode != InteractionMode::MoveSphere {
            return None;
        }
        
        let ndc = self.screen_to_ndc(self.mouse_pos.x, self.mouse_pos.y);
        let ray = self.get_ray(ndc, view_proj_inv);
        
        // Intersect with drag plane
        if let Some(hit) = self.ray_plane_intersection(ray.0, ray.1, self.prev_hit, self.plane_normal) {
            let delta = hit - self.prev_hit;
            self.prev_hit = hit;
            Some(delta)
        } else {
            None
        }
    }
    
    /// Convert screen coords to NDC
    fn screen_to_ndc(&self, x: f32, y: f32) -> Vec2 {
        Vec2::new(
            (x / self.screen_size.x) * 2.0 - 1.0,
            -((y / self.screen_size.y) * 2.0 - 1.0),
        )
    }
    
    /// Get ray from camera through NDC point
    fn get_ray(&self, ndc: Vec2, view_proj_inv: Mat4) -> (Vec3, Vec3) {
        // Near and far points in NDC
        let near_ndc = Vec4::new(ndc.x, ndc.y, 0.0, 1.0);
        let far_ndc = Vec4::new(ndc.x, ndc.y, 1.0, 1.0);
        
        // Transform to world space
        let near_world = view_proj_inv * near_ndc;
        let far_world = view_proj_inv * far_ndc;
        
        let near = near_world.truncate() / near_world.w;
        let far = far_world.truncate() / far_world.w;
        
        let direction = (far - near).normalize();
        
        (near, direction)
    }
    
    /// Raycast to water plane (y = 0)
    fn raycast_water_plane(&self, ndc: Vec2, view_proj_inv: Mat4) -> Option<Vec3> {
        let (origin, dir) = self.get_ray(ndc, view_proj_inv);
        self.ray_plane_intersection(origin, dir, Vec3::ZERO, Vec3::Y)
    }
    
    /// Ray-plane intersection
    fn ray_plane_intersection(&self, origin: Vec3, dir: Vec3, plane_point: Vec3, plane_normal: Vec3) -> Option<Vec3> {
        let denom = dir.dot(plane_normal);
        if denom.abs() < 1e-6 {
            return None;
        }
        
        let t = (plane_point - origin).dot(plane_normal) / denom;
        if t > 0.0 {
            Some(origin + dir * t)
        } else {
            None
        }
    }
    
    /// Ray-sphere intersection
    fn ray_sphere_intersection(&self, origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<Vec3> {
        let to_sphere = origin - center;
        let a = dir.dot(dir);
        let b = 2.0 * to_sphere.dot(dir);
        let c = to_sphere.dot(to_sphere) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;
        
        if discriminant > 0.0 {
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            if t > 0.0 {
                return Some(origin + dir * t);
            }
        }
        None
    }
    
    /// Update screen size
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_size = Vec2::new(width, height);
    }
}
