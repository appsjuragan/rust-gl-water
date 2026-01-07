//! Physics module - handles buoyancy and collision physics for floating objects

use glam::Vec3;

/// Physics engine for floating sphere (duck) simulation
pub struct PhysicsEngine {
    /// Current center position of sphere
    pub center: Vec3,
    /// Previous center position (for water interaction)
    pub old_center: Vec3,
    /// Current velocity
    pub velocity: Vec3,
    /// Gravity vector
    pub gravity: Vec3,
    /// Sphere radius
    pub radius: f32,
    
    // Pool dimensions
    pub pool_width: f32,
    pub pool_length: f32,
    pub pool_depth: f32,
    
    // Physics parameters
    /// What fraction of sphere floats above water (0-1)
    pub float_ratio: f32,
    /// How much the sphere disturbs water
    pub impact_strength: f32,
    /// Whether physics simulation is enabled
    pub enabled: bool,
    /// Whether gravity is applied
    pub gravity_enabled: bool,
}

impl Default for PhysicsEngine {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            old_center: Vec3::ZERO,
            velocity: Vec3::ZERO,
            gravity: Vec3::new(0.0, -9.81, 0.0), // Use Earth gravity
            radius: 0.25,
            pool_width: 2.0,
            pool_length: 2.0,
            pool_depth: 0.0,
            float_ratio: 0.5, // 50% submerged
            impact_strength: 0.04,
            enabled: true,
            gravity_enabled: true,
        }
    }
}

impl PhysicsEngine {
    /// Update physics simulation
    /// 
    /// # Arguments
    /// * `dt` - Delta time in seconds
    /// * `water_height` - Water height at sphere position
    /// * `mouse_point` - Optional mouse point for repulsion
    /// * `is_dragging` - Whether user is dragging the sphere
    pub fn update(
        &mut self,
        dt: f32,
        water_height: f32,
        mouse_point: Option<Vec3>,
        is_dragging: bool,
    ) {
        // Skip if time step is too large
        if dt > 1.0 {
            return;
        }
        
        // Store old position
        self.old_center = self.center;
        
        if is_dragging {
            // User is manually moving sphere
            self.velocity = Vec3::ZERO;
        } else if self.enabled {
            // Calculate how much of sphere is underwater
            let percent_underwater = ((water_height + self.radius - self.center.y) 
                / (2.0 * self.radius))
                .clamp(0.0, 1.0);
            
            let buoyancy_factor = 1.0 / (1.0 - self.float_ratio);
            
            // Apply gravity and buoyancy (vertical)
            let effective_gravity = if self.gravity_enabled { self.gravity } else { Vec3::ZERO };
            let g_term = effective_gravity * (dt - buoyancy_factor * dt * percent_underwater);
            self.velocity += g_term;
            
            // Mouse repulsion (horizontal)
            if let Some(mouse) = mouse_point {
                let mut dist_vec = self.center - mouse;
                dist_vec.y = 0.0; // Horizontal only
                let dist = dist_vec.length();
                let influence_radius = 1.0;
                
                if dist < influence_radius && dist > 0.001 {
                    let push_strength = 2.0;
                    let force = dist_vec.normalize() 
                        * push_strength 
                        * (1.0 - dist / influence_radius) 
                        * dt;
                    self.velocity += force;
                }
            }
            
            // Apply drag based on underwater percentage
            if self.velocity.length_squared() > 0.0 {
                let drag = self.velocity.normalize()
                    * percent_underwater
                    * dt
                    * self.velocity.dot(self.velocity);
                self.velocity -= drag;
            }
            
            // Integrate position
            self.center += self.velocity * dt;
            
            // Wall collisions (X)
            let half_width = self.pool_width / 2.0;
            if self.center.x < self.radius - half_width {
                self.center.x = self.radius - half_width;
                self.velocity.x = self.velocity.x.abs() * 0.5;
            } else if self.center.x > half_width - self.radius {
                self.center.x = half_width - self.radius;
                self.velocity.x = -self.velocity.x.abs() * 0.5;
            }
            
            // Wall collisions (Z)
            let half_length = self.pool_length / 2.0;
            if self.center.z < self.radius - half_length {
                self.center.z = self.radius - half_length;
                self.velocity.z = self.velocity.z.abs() * 0.5;
            } else if self.center.z > half_length - self.radius {
                self.center.z = half_length - self.radius;
                self.velocity.z = -self.velocity.z.abs() * 0.5;
            }
            
            // Floor collision
            if self.center.y < self.radius - self.pool_depth {
                self.center.y = self.radius - self.pool_depth;
                self.velocity.y = self.velocity.y.abs() * 0.7;
            }
        }
    }
    
    /// Move sphere by a delta (for mouse dragging)
    pub fn move_by(&mut self, delta: Vec3) {
        self.center += delta;
        
        // Clamp to pool bounds
        let half_width = self.pool_width / 2.0;
        let half_length = self.pool_length / 2.0;
        
        self.center.x = self.center.x.clamp(
            self.radius - half_width,
            half_width - self.radius,
        );
        self.center.y = self.center.y.clamp(
            self.radius - self.pool_depth,
            10.0,
        );
        self.center.z = self.center.z.clamp(
            self.radius - half_length,
            half_length - self.radius,
        );
    }
    
    /// Get sphere displacement for water simulation
    pub fn get_displacement(&self) -> Vec3 {
        self.center - self.old_center
    }
}

/// Uniform data for physics objects in shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereUniform {
    pub center: [f32; 4],
    pub radius: f32,
    pub _padding: [f32; 3],
}

impl Default for SphereUniform {
    fn default() -> Self {
        Self {
            center: [0.0; 4],
            radius: 0.25,
            _padding: [0.0; 3],
        }
    }
}

impl SphereUniform {
    pub fn update(&mut self, physics: &PhysicsEngine) {
        self.center = [physics.center.x, physics.center.y, physics.center.z, 1.0];
        self.radius = physics.radius;
    }
}
