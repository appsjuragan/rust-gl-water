//! Physics module - handles buoyancy and collision physics for floating objects

use glam::{Vec3, Quat};
use crate::gui::PoolShape;

#[derive(Clone, Copy, Debug)]
pub struct ObjectState {
    pub center: Vec3,
    pub old_center: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub angular_velocity: Vec3,
}

impl Default for ObjectState {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            old_center: Vec3::ZERO,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
        }
    }
}

/// Physics engine for floating simulation
pub struct PhysicsEngine {
    pub objects: Vec<ObjectState>,
    
    /// Gravity vector
    pub gravity: Vec3,
    /// Sphere radius
    pub radius: f32,
    
    // Pool dimensions
    pub pool_width: f32,
    pub pool_length: f32,
    pub pool_depth: f32,
    pub pool_shape: PoolShape,
    
    // Physics parameters
    pub float_ratio: f32,
    pub impact_strength: f32,
    pub enabled: bool,
    pub gravity_enabled: bool,
    pub mouse_repulsion_enabled: bool,
    
    // Random seed
    pub seed: u32,
}

impl Default for PhysicsEngine {
    fn default() -> Self {
        // Initialize with one default object
        let mut objects = Vec::new();
        objects.push(ObjectState {
            center: Vec3::new(0.0, 2.0, 0.0),
            old_center: Vec3::new(0.0, 2.0, 0.0),
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
        });

        Self {
            objects,
            gravity: Vec3::new(0.0, -9.81, 0.0), // Use Earth gravity
            radius: 0.25,
            pool_width: 2.0,
            pool_length: 2.0,
            pool_depth: 0.0,
            pool_shape: PoolShape::Cube,
            float_ratio: 0.5, // 50% submerged
            impact_strength: 0.04,
            enabled: true,
            gravity_enabled: true,
            mouse_repulsion_enabled: true,
            seed: 12345,
        }
    }
}

impl PhysicsEngine {
    fn random_f32_static(seed: &mut u32) -> f32 {
        *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (*seed as f32) / (u32::MAX as f32)
    }

    /// Update physics simulation for all objects
    pub fn update(
        &mut self,
        dt: f32,
        water_height: f32,
        mouse_point: Option<Vec3>,
        dragged_object_index: Option<usize>,
    ) {
        // Skip if time step is too large
        if dt > 1.0 {
            return;
        }
        
        // Extract fields to avoid borrowing self in loop
        let gravity = self.gravity;
        let radius = self.radius;
        let pool_width = self.pool_width;
        let pool_length = self.pool_length;
        let pool_depth = self.pool_depth;
        let pool_shape = self.pool_shape;
        let float_ratio = self.float_ratio;
        let enabled = self.enabled;
        let gravity_enabled = self.gravity_enabled;
        let mouse_repulsion_enabled = self.mouse_repulsion_enabled;
        let seed = &mut self.seed;
        
        // 1. Update individual objects
        for (i, obj) in self.objects.iter_mut().enumerate() {
            // Store old position
            obj.old_center = obj.center;
            
            // If this object is being dragged, skip physics integration
            if Some(i) == dragged_object_index {
                obj.velocity = Vec3::ZERO;
                obj.angular_velocity = Vec3::ZERO;
                continue;
            } else if !enabled {
                continue;
            }

            // 1. Calculate forces
            let mut force = Vec3::ZERO;

            // Gravity
            if gravity_enabled {
                force += gravity;
            }

            // Buoyancy
            // percent_underwater = 0 at y = center + radius, 1 at y = center - radius
            let submerged_depth = (water_height + radius - obj.center.y).max(0.0);
            let percent_underwater = (submerged_depth / (2.0 * radius)).min(1.0);
            
            // Equilibrium at float_ratio: Gravity + Buoyancy = 0
            // Buoyancy = -gravity * (percent / float_ratio)
            if percent_underwater > 0.0 {
                let buoyancy_force = -gravity * (percent_underwater / float_ratio);
                force += buoyancy_force;
                
                // Add vertical damping (viscosity)
                force -= obj.velocity * (percent_underwater * 2.0);
                
                // Random torque from waves
                let rx = (Self::random_f32_static(seed) - 0.5) * 2.0;
                let ry = (Self::random_f32_static(seed) - 0.5) * 2.0;
                let rz = (Self::random_f32_static(seed) - 0.5) * 2.0;
                let random_torque = Vec3::new(rx, ry, rz).normalize_or_zero() * 2.0 * percent_underwater;
                obj.angular_velocity += random_torque * dt;
            }

            // Mouse repulsion (horizontal) - can be toggled with 'L' key
            if mouse_repulsion_enabled {
                if let Some(mouse) = mouse_point {
                    let mut dist_vec = obj.center - mouse;
                    dist_vec.y = 0.0;
                    let dist = dist_vec.length();
                    let influence_radius = 1.0;
                    
                    if dist < influence_radius && dist > 0.001 {
                        let push_strength = 15.0;
                        let push_force = dist_vec.normalize() 
                            * push_strength 
                            * (1.0 - dist / influence_radius);
                        force += push_force;
                    }
                }
            }
            
            // Apply drag (quadratic)
            if obj.velocity.length_squared() > 0.001 {
                let speed = obj.velocity.length();
                let drag_coeff = 0.5 + percent_underwater * 2.0;
                let drag_force = -obj.velocity.normalize() * (speed * speed * drag_coeff);
                force += drag_force;
                
                // Induce rolling from movement (drag acts on surface, not center)
                // Torque = r x F
                // Assume drag acts on bottom if floating, or opposite to velocity
                // Simplified: Roll axis is cross product of Up and Velocity
                let roll_axis = Vec3::Y.cross(obj.velocity).normalize_or_zero();
                obj.angular_velocity += roll_axis * speed * 2.0 * dt;
            }
            
            // 2. Integrate (Semi-Implicit Euler)
            obj.velocity += force * dt;
            obj.center += obj.velocity * dt;

            // Add some global damping to prevent infinite energy
            obj.velocity *= 0.995;
            
            // Angular damping
            obj.angular_velocity *= 0.98;
            
            // Integrate rotation
            let angle = obj.angular_velocity.length();
            if angle > 0.0001 {
                let axis = obj.angular_velocity / angle;
                let rot_delta = Quat::from_axis_angle(axis, angle * dt);
                obj.rotation = (rot_delta * obj.rotation).normalize();
            }
            
            // Wall collisions
            match pool_shape {
                PoolShape::Cylinder => {
                    let dist_sq = obj.center.x * obj.center.x + obj.center.z * obj.center.z;
                    let max_dist = pool_width / 2.0 - radius;
                    
                    if dist_sq > max_dist * max_dist {
                        let dist = dist_sq.sqrt();
                        if dist > 0.0001 {
                            let normal = Vec3::new(obj.center.x, 0.0, obj.center.z) / dist;
                            obj.center.x = normal.x * max_dist;
                            obj.center.z = normal.z * max_dist;
                            
                            // Reflect velocity
                            let v_dot_n = obj.velocity.dot(normal);
                            if v_dot_n > 0.0 {
                                obj.velocity -= normal * (v_dot_n * 1.5); // Bounce
                                
                                // Add friction torque
                                let tangent = (obj.velocity - normal * v_dot_n).normalize_or_zero();
                                let torque_axis = normal.cross(tangent);
                                obj.angular_velocity += torque_axis * v_dot_n * 5.0;
                            }
                        }
                    }
                },
                PoolShape::Frustum => {
                    // Approximate frustum collision
                    let h_total = 1.4;
                    let y_rel = obj.center.y + 1.0; // relative to bottom
                    let t = (y_rel / h_total).clamp(0.0, 1.0);
                    let scale = 0.7 + (1.0 - 0.7) * t;
                    
                    let half_width = (pool_width / 2.0) * scale;
                    let half_length = (pool_length / 2.0) * scale;
                    
                    Self::collide_box(obj, half_width, half_length, radius);
                },
                _ => {
                    let half_width = pool_width / 2.0;
                    let half_length = pool_length / 2.0;
                    Self::collide_box(obj, half_width, half_length, radius);
                }
            }
            
            // Floor collision
            if obj.center.y < radius - pool_depth {
                obj.center.y = radius - pool_depth;
                obj.velocity.y = obj.velocity.y.abs() * 0.7;
                
                // Floor friction
                let v_horiz = Vec3::new(obj.velocity.x, 0.0, obj.velocity.z);
                if v_horiz.length() > 0.01 {
                    let roll_axis = Vec3::Y.cross(v_horiz).normalize();
                    obj.angular_velocity += roll_axis * v_horiz.length() * 5.0 * dt;
                }
            }
        }

        // 2. Solve Object-Object Collisions
        self.solve_object_collisions();
    }
    
    fn collide_box(obj: &mut ObjectState, half_width: f32, half_length: f32, radius: f32) {
        if obj.center.x < radius - half_width {
            obj.center.x = radius - half_width;
            obj.velocity.x = obj.velocity.x.abs() * 0.5;
            // Friction/Torque
            obj.angular_velocity.z -= obj.velocity.y * 2.0;
        } else if obj.center.x > half_width - radius {
            obj.center.x = half_width - radius;
            obj.velocity.x = -obj.velocity.x.abs() * 0.5;
            obj.angular_velocity.z += obj.velocity.y * 2.0;
        }
        
        if obj.center.z < radius - half_length {
            obj.center.z = radius - half_length;
            obj.velocity.z = obj.velocity.z.abs() * 0.5;
            obj.angular_velocity.x += obj.velocity.y * 2.0;
        } else if obj.center.z > half_length - radius {
            obj.center.z = half_length - radius;
            obj.velocity.z = -obj.velocity.z.abs() * 0.5;
            obj.angular_velocity.x -= obj.velocity.y * 2.0;
        }
    }
    
    fn solve_object_collisions(&mut self) {
        let count = self.objects.len();
        if count < 2 { return; }
        
        // Simple distinct pair iteration
        for i in 0..count {
            for j in (i + 1)..count {
                let p1 = self.objects[i].center;
                let p2 = self.objects[j].center;
                let diff = p1 - p2;
                let dist_sq = diff.length_squared();
                let min_dist = self.radius * 2.0; // Assume same radius
                
                if dist_sq < min_dist * min_dist {
                    let dist = dist_sq.sqrt();
                    if dist < 0.0001 { continue; } // Avoid division by zero
                    
                    let overlap = min_dist - dist;
                    let normal = diff / dist;
                    
                    // Separate objects
                    let correction = normal * (overlap * 0.5);
                    self.objects[i].center += correction;
                    self.objects[j].center -= correction;
                    
                    // Exchange momentum (Elastic collision approximation)
                    // v1' = v1 - dot(v1-v2, n) * n
                    // v2' = v2 - dot(v2-v1, n) * n(but n is p1-p2, so for v2 use -n)
                    
                    let v1 = self.objects[i].velocity;
                    let v2 = self.objects[j].velocity;
                    
                    let relative_vel = v1 - v2;
                    let speed = relative_vel.dot(normal);
                    
                    if speed < 0.0 { // Closing in
                        let impulse = normal * speed * 1.5; // 1.5 for bounce
                        self.objects[i].velocity -= impulse * 0.5;
                        self.objects[j].velocity += impulse * 0.5;
                        
                        // Transfer angular momentum (friction)
                        let tangent = (relative_vel - normal * speed).normalize_or_zero();
                        let torque_axis = normal.cross(tangent);
                        let torque = torque_axis * speed.abs() * 2.0;
                        self.objects[i].angular_velocity += torque;
                        self.objects[j].angular_velocity -= torque;
                    }
                }
            }
        }
    }
    
    /// Move specific object by a delta
    pub fn move_by(&mut self, index: usize, delta: Vec3) {
        if index >= self.objects.len() { return; }
        
        let obj = &mut self.objects[index];
        obj.center += delta;
        
        // Clamp to pool bounds
        match self.pool_shape {
            PoolShape::Cylinder => {
                let max_dist = self.pool_width / 2.0 - self.radius;
                let dist_sq = obj.center.x * obj.center.x + obj.center.z * obj.center.z;
                if dist_sq > max_dist * max_dist {
                    let dist = dist_sq.sqrt();
                    if dist > 0.0001 {
                        let normal = Vec3::new(obj.center.x, 0.0, obj.center.z) / dist;
                        obj.center.x = normal.x * max_dist;
                        obj.center.z = normal.z * max_dist;
                    }
                }
            },
            _ => {
                let half_width = self.pool_width / 2.0;
                let half_length = self.pool_length / 2.0;
                
                obj.center.x = obj.center.x.clamp(
                    self.radius - half_width,
                    half_width - self.radius,
                );
                obj.center.z = obj.center.z.clamp(
                    self.radius - half_length,
                    half_length - self.radius,
                );
            }
        }
        
        obj.center.y = obj.center.y.clamp(
            self.radius - self.pool_depth,
            10.0,
        );
    }
    
    pub fn reset_objects(&mut self, count: usize) {
        self.objects.clear();
        for i in 0..count {
            // Scatter objects slightly
            let offset_x = (i as f32 % 3.0 - 1.0) * 0.5;
            let offset_z = ((i / 3) as f32 - 0.5) * 0.5;
            
            self.objects.push(ObjectState {
                center: Vec3::new(offset_x, 2.0 + (i as f32) * 0.5, offset_z), // Drop from height
                old_center: Vec3::new(offset_x, 2.0 + (i as f32) * 0.5, offset_z),
                velocity: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                angular_velocity: Vec3::ZERO,
            });
        }
    }
}
