//! Physics module - handles buoyancy and collision physics for floating objects

use glam::{Vec3, Quat};
use crate::gui::PoolShape;
use std::sync::Arc;
use crate::core::shape::{Shape, ShapeParams};
use crate::core::physics_trait::PhysicsState;

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

#[derive(Clone, Copy, Debug)]
pub struct RippleEvent {
    pub x: f32,
    pub z: f32,
    pub strength: f32,
    pub radius: f32,
}

/// Physics engine for floating simulation
pub struct PhysicsEngine {
    pub objects: Vec<ObjectState>,
    
    pub gravity: Vec3,
    pub radius: f32,
    
    pub pool_width: f32,
    pub pool_length: f32,
    pub pool_depth: f32,
    pub wall_height: f32,
    pub pool_shape: PoolShape,
    

    pub impact_strength: f32,
    pub enabled: bool,
    pub gravity_enabled: bool,
    pub mouse_repulsion_enabled: bool,
    
    pub seed: u32,

    pub current_shape: Arc<dyn Shape>,
    
    /// Material density relative to water (1.0 = water)
    /// < 1.0: floats, > 1.0: sinks
    pub material_density: f32,
    
    /// Queue of ripple events to be rendered
    pub ripples: Vec<RippleEvent>,
}

impl Default for PhysicsEngine {
    fn default() -> Self {
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
            gravity: Vec3::new(0.0, -9.81, 0.0),
            radius: 0.25,
            pool_width: 2.0,
            pool_length: 2.0,
            pool_depth: 1.0,
            wall_height: 0.4,
            pool_shape: PoolShape::Cube,

            impact_strength: 0.04,
            enabled: true,
            gravity_enabled: true,
            mouse_repulsion_enabled: true,
            seed: 12345,
            current_shape: Arc::new(crate::shapes::Sphere::new()),
            material_density: 0.6, // Wood by default
            ripples: Vec::new(),
        }
    }
}

impl PhysicsEngine {
    pub fn set_shape(&mut self, shape_name: &str) {
        let registry = crate::shapes::create_default_registry();
        if let Some(shape) = registry.get(shape_name) {
            self.current_shape = shape;
        }
    }

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
        // float_ratio replaced by material_density
        let enabled = self.enabled;
        let gravity_enabled = self.gravity_enabled;
        let mouse_repulsion_enabled = self.mouse_repulsion_enabled;
        let seed = &mut self.seed;
        
        let material_density = self.material_density;
        
        self.ripples.clear();
        
        for (i, obj) in self.objects.iter_mut().enumerate() {
            obj.old_center = obj.center;
            
            if Some(i) == dragged_object_index {
                obj.velocity = Vec3::ZERO;
                obj.angular_velocity = Vec3::ZERO;
                continue;
            } else if !enabled {
                continue;
            }

            let mut force = Vec3::ZERO;

            if gravity_enabled {
                force += gravity * material_density;
            }

            // Buoyancy based on material density
            // At equilibrium: submerged_fraction = material_density (when density < 1.0)
            // If density > 1.0, object sinks (buoyancy < weight)
            let submerged_depth = (water_height + radius - obj.center.y).max(0.0);
            let percent_underwater = (submerged_depth / (2.0 * radius)).min(1.0);
            
            if percent_underwater > 0.0 {
                // Buoyancy = water_density * g * submerged_volume
                // Weight = material_density * g * total_volume
                // Net force = (buoyancy - weight) = g * (submerged_fraction - material_density)
                let buoyancy_force = -gravity * percent_underwater;
                force += buoyancy_force;
                
                // Damping increases with submersion
                let damping = if material_density > 1.0 { 4.0 } else { 2.0 };
                force -= obj.velocity * (percent_underwater * damping);
                
                let rx = (Self::random_f32_static(seed) - 0.5) * 2.0;
                let ry = (Self::random_f32_static(seed) - 0.5) * 2.0;
                let rz = (Self::random_f32_static(seed) - 0.5) * 2.0;
                
                // Only apply random torque to floating objects
                if material_density <= 1.0 {
                    let random_torque = Vec3::new(rx, ry, rz).normalize_or_zero() * 1.0 * percent_underwater;
                    obj.angular_velocity += random_torque * dt;
                }
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
                let roll_mult = if material_density > 1.0 { 0.2 } else { 1.0 };
                obj.angular_velocity += roll_axis * speed * roll_mult * dt;
            }
            
            // 2. Integrate (Semi-Implicit Euler)
            obj.velocity += force * dt;
            obj.center += obj.velocity * dt;

            // Ripple Generation
            // 1. Impact Splash
            let was_submerged = (obj.old_center.y - radius) < water_height;
            let is_submerged = (obj.center.y - radius) < water_height;
            
            if !was_submerged && is_submerged && obj.velocity.y < -0.1 {
                 self.ripples.push(RippleEvent {
                     x: obj.center.x,
                     z: obj.center.z,
                     strength: -0.05 * obj.velocity.y.abs().min(5.0),
                     radius: radius * 1.5,
                 });
            }

            // 2. Sinking Ripples (for dense objects)
            if material_density > 1.0 && percent_underwater > 0.0 && percent_underwater < 0.9 {
                 if Self::random_f32_static(seed) < 0.08 {
                     self.ripples.push(RippleEvent {
                         x: obj.center.x + (Self::random_f32_static(seed) - 0.5) * radius * 0.5,
                         z: obj.center.z + (Self::random_f32_static(seed) - 0.5) * radius * 0.5,
                         strength: -0.015,
                         radius: radius * 0.4,
                     });
                 }
            }

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
            let collider = self.current_shape.as_ref().collider();
            let shape_params = ShapeParams {
                radius: self.radius,
                rotation: obj.rotation,
                scale: Vec3::splat(self.radius), // Apply radius as scale
            };

            // Create PhysicsState adapter
            let mut state = PhysicsState {
                position: obj.center,
                velocity: obj.velocity,
                rotation: obj.rotation,
                angular_velocity: obj.angular_velocity,
                mass: 1.0, // Default mass
            };

            match pool_shape {
                PoolShape::Tube => {
                    collider.collide_with_cylinder(
                        &mut state,
                        pool_width / 2.0,
                        self.current_shape.as_ref(),
                        &shape_params,
                    );
                },
                PoolShape::Frustum => {
                    // Assuming top scale 1.0, bottom 0.7
                    collider.collide_with_frustum(
                        &mut state,
                        1.0,
                        0.7,
                        -pool_depth,
                        self.wall_height,
                        self.current_shape.as_ref(),
                        &shape_params,
                    );
                },
                _ => {
                    let half_width = pool_width / 2.0;
                    let half_length = pool_length / 2.0;
                    collider.collide_with_box(
                        &mut state,
                        half_width,
                        half_length,
                        self.current_shape.as_ref(),
                        &shape_params,
                    );
                }
            }
            
            // Sync back to ObjectState
            obj.center = state.position;
            obj.velocity = state.velocity;
            obj.rotation = state.rotation;
            obj.angular_velocity = state.angular_velocity;
            
            // Floor collision using shape-specific collider
            let floor_y = -pool_depth;
            let mut floor_state = PhysicsState {
                position: obj.center,
                velocity: obj.velocity,
                rotation: obj.rotation,
                angular_velocity: obj.angular_velocity,
                mass: 1.0,
            };
            
            collider.collide_with_floor(
                &mut floor_state,
                floor_y,
                self.current_shape.as_ref(),
                &shape_params,
                dt,
            );
            
            obj.center = floor_state.position;
            obj.velocity = floor_state.velocity;
            obj.rotation = floor_state.rotation;
            obj.angular_velocity = floor_state.angular_velocity;
        }

        // 2. Solve Object-Object Collisions
        self.solve_object_collisions();
    }

    

    /// Iterative Position-Based Dynamics (PBD) solver for stable stacking
    fn solve_object_collisions(&mut self) {
        let count = self.objects.len();
        if count < 2 { return; }
        
        let collider = self.current_shape.as_ref().collider();
        
        // Store positions before solving for velocity derivation
        let positions_before: Vec<Vec3> = self.objects.iter().map(|o| o.center).collect();
        
        // PBD: Multiple iterations for constraint convergence
        const SOLVER_ITERATIONS: usize = 8;
        const POSITION_SLOP: f32 = 0.001; // Allowed penetration
        
        for _iter in 0..SOLVER_ITERATIONS {
            // Collect all collision pairs and their corrections first (Jacobi-style)
            let mut corrections: Vec<(usize, Vec3)> = Vec::new();
            
            for i in 0..count {
                for j in (i + 1)..count {
                    let shape_params_a = ShapeParams {
                        radius: self.radius,
                        rotation: self.objects[i].rotation,
                        scale: Vec3::splat(self.radius),
                    };
                    let shape_params_b = ShapeParams {
                        radius: self.radius,
                        rotation: self.objects[j].rotation,
                        scale: Vec3::splat(self.radius),
                    };

                    let state_a = PhysicsState {
                        position: self.objects[i].center,
                        velocity: Vec3::ZERO, // Not used for position solving
                        rotation: self.objects[i].rotation,
                        angular_velocity: Vec3::ZERO,
                        mass: 1.0,
                    };
                    
                    let state_b = PhysicsState {
                        position: self.objects[j].center,
                        velocity: Vec3::ZERO,
                        rotation: self.objects[j].rotation,
                        angular_velocity: Vec3::ZERO,
                        mass: 1.0,
                    };

                    if let Some(collision) = collider.check_object_collision(
                        &state_a,
                        &state_b,
                        self.current_shape.as_ref(),
                        self.current_shape.as_ref(),
                        &shape_params_a,
                        &shape_params_b,
                    ) {
                        let penetration = collision.penetration_depth;
                        
                        if penetration > POSITION_SLOP {
                            let normal = collision.normal;
                            // Baumgarte stabilization factor (0.1-0.3 typical)
                            let baumgarte = 0.2;
                            let correction_mag = (penetration - POSITION_SLOP) * baumgarte;
                            let correction = normal * correction_mag;
                            
                            // Equal mass assumption: split 50/50
                            corrections.push((i, -correction));
                            corrections.push((j, correction));
                        }
                    }
                }
            }
            
            // Apply all corrections (Jacobi: use averaged corrections)
            let mut accumulated: Vec<(Vec3, u32)> = vec![(Vec3::ZERO, 0); count];
            for (idx, corr) in corrections {
                accumulated[idx].0 += corr;
                accumulated[idx].1 += 1;
            }
            for (i, (total_corr, num)) in accumulated.into_iter().enumerate() {
                if num > 0 {
                    self.objects[i].center += total_corr / num as f32;
                }
            }
        }
        
        // After position solving: derive velocity from position change
        // and apply proper impulse-based collision response
        for i in 0..count {
            for j in (i + 1)..count {
                let shape_params_a = ShapeParams {
                    radius: self.radius,
                    rotation: self.objects[i].rotation,
                    scale: Vec3::splat(self.radius),
                };
                let shape_params_b = ShapeParams {
                    radius: self.radius,
                    rotation: self.objects[j].rotation,
                    scale: Vec3::splat(self.radius),
                };

                let state_a = PhysicsState {
                    position: self.objects[i].center,
                    velocity: self.objects[i].velocity,
                    rotation: self.objects[i].rotation,
                    angular_velocity: self.objects[i].angular_velocity,
                    mass: 1.0,
                };
                
                let state_b = PhysicsState {
                    position: self.objects[j].center,
                    velocity: self.objects[j].velocity,
                    rotation: self.objects[j].rotation,
                    angular_velocity: self.objects[j].angular_velocity,
                    mass: 1.0,
                };

                if let Some(collision) = collider.check_object_collision(
                    &state_a,
                    &state_b,
                    self.current_shape.as_ref(),
                    self.current_shape.as_ref(),
                    &shape_params_a,
                    &shape_params_b,
                ) {
                    let normal = collision.normal;
                    let v1 = self.objects[i].velocity;
                    let v2 = self.objects[j].velocity;
                    let relative_vel = v1 - v2;
                    let closing_speed = relative_vel.dot(normal);
                    
                    // Only apply impulse if objects are approaching
                    if closing_speed < -0.001 {
                        // Restitution: 0.0 = perfectly inelastic, 1.0 = perfectly elastic
                        // Use low restitution for stable stacking
                        let restitution = 0.1;
                        let impulse_mag = -(1.0 + restitution) * closing_speed * 0.5;
                        let impulse = normal * impulse_mag;
                        
                        self.objects[i].velocity += impulse;
                        self.objects[j].velocity -= impulse;
                        
                        // Friction
                        let tangent_vel = relative_vel - normal * closing_speed;
                        if tangent_vel.length_squared() > 1e-6 {
                            let friction_coeff = 0.4;
                            let max_friction = impulse_mag * friction_coeff;
                            let friction_impulse = tangent_vel.normalize() * tangent_vel.length().min(max_friction);
                            
                            self.objects[i].velocity -= friction_impulse * 0.5;
                            self.objects[j].velocity += friction_impulse * 0.5;
                            
                            // Rolling resistance
                            let torque_axis = normal.cross(tangent_vel).normalize_or_zero();
                            let torque = torque_axis * tangent_vel.length() * 0.3;
                            self.objects[i].angular_velocity += torque;
                            self.objects[j].angular_velocity -= torque;
                        }
                    } else {
                        // Resting contact: apply strong damping
                        let contact_damping = 0.92;
                        // Only damp the component along the contact normal
                        let v1_n = self.objects[i].velocity.dot(normal);
                        let v2_n = self.objects[j].velocity.dot(normal);
                        self.objects[i].velocity -= normal * v1_n * (1.0 - contact_damping);
                        self.objects[j].velocity -= normal * v2_n * (1.0 - contact_damping);
                        
                        self.objects[i].angular_velocity *= 0.95;
                        self.objects[j].angular_velocity *= 0.95;
                    }
                }
            }
        }
        
        // Velocity threshold: objects moving very slowly should stop
        const SLEEP_VELOCITY_THRESHOLD: f32 = 0.005;
        const SLEEP_ANGULAR_THRESHOLD: f32 = 0.01;
        
        for obj in &mut self.objects {
            if obj.velocity.length_squared() < SLEEP_VELOCITY_THRESHOLD * SLEEP_VELOCITY_THRESHOLD {
                obj.velocity = Vec3::ZERO;
            }
            if obj.angular_velocity.length_squared() < SLEEP_ANGULAR_THRESHOLD * SLEEP_ANGULAR_THRESHOLD {
                obj.angular_velocity = Vec3::ZERO;
            }
        }
        
        // Suppress unused variable warning
        let _ = positions_before;
    }
    
    /// Move specific object by a delta
    pub fn move_by(&mut self, index: usize, delta: Vec3) {
        if index >= self.objects.len() { return; }
        
        let obj = &mut self.objects[index];
        obj.center += delta;
        
        // Clamp to pool bounds
        // Clamp to pool bounds
        let collider = self.current_shape.as_ref().collider();
        let shape_params = ShapeParams {
            radius: self.radius,
            rotation: obj.rotation,
            scale: Vec3::splat(self.radius),
        };
        
        let mut state = PhysicsState {
            position: obj.center,
            velocity: Vec3::ZERO,
            rotation: obj.rotation,
            angular_velocity: Vec3::ZERO,
            mass: 1.0,
        };

        match self.pool_shape {
            PoolShape::Tube => {
                collider.collide_with_cylinder(
                    &mut state,
                    self.pool_width / 2.0,
                    self.current_shape.as_ref(),
                    &shape_params,
                );
            },
            PoolShape::Frustum => {
                collider.collide_with_frustum(
                    &mut state,
                    1.0, 
                    0.7, 
                    -self.pool_depth,
                    self.wall_height,
                    self.current_shape.as_ref(), 
                    &shape_params,
                );
            },
            _ => {
                let half_width = self.pool_width / 2.0;
                let half_length = self.pool_length / 2.0;
                collider.collide_with_box(
                    &mut state,
                    half_width,
                    half_length,
                    self.current_shape.as_ref(),
                    &shape_params,
                );
            }
        }
        
        obj.center = state.position;
        
        obj.center.y = obj.center.y.clamp(
            self.radius - self.pool_depth,
            10.0,
        );
    }
    
    pub fn reset_objects(&mut self, count: usize) {
        self.objects.clear();
        let grid_size = (count as f32).sqrt().ceil().max(1.0) as usize;
        let spacing = 0.4;
        let offset = (grid_size as f32 - 1.0) * spacing * 0.5;
        
        for i in 0..count {
            let row = i / grid_size;
            let col = i % grid_size;
            let offset_x = (col as f32) * spacing - offset;
            let offset_z = (row as f32) * spacing - offset;
            
            self.objects.push(ObjectState {
                center: Vec3::new(offset_x, 1.5 + (i as f32) * 0.2, offset_z), // Drop from height
                old_center: Vec3::new(offset_x, 1.5 + (i as f32) * 0.2, offset_z),
                velocity: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                angular_velocity: Vec3::ZERO,
            });
        }
    }
}
