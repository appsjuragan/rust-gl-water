//! Physics module - handles buoyancy and collision physics for floating objects

use glam::{Vec3, Quat};
use crate::core::enums::PoolShapeType;
use std::sync::Arc;
use crate::core::shape::{Shape, ShapeParams};
use crate::core::state::RigidBody;
use crate::core::constants::{
    DEFAULT_GRAVITY, DEFAULT_OBJECT_RADIUS, POOL_SIZE_DEFAULT, POOL_DEPTH, WALL_HEIGHT,
};

pub type ObjectState = RigidBody;

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
    pub pool_shape: PoolShapeType,
    
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
            position: Vec3::new(0.0, 2.0, 0.0),
            old_position: Vec3::new(0.0, 2.0, 0.0),
            ..Default::default()
        });

        Self {
            objects,
            gravity: Vec3::new(0.0, -DEFAULT_GRAVITY, 0.0),
            radius: DEFAULT_OBJECT_RADIUS,
            pool_width: POOL_SIZE_DEFAULT,
            pool_length: POOL_SIZE_DEFAULT,
            pool_depth: POOL_DEPTH,
            wall_height: WALL_HEIGHT,
            pool_shape: PoolShapeType::Cube,

            impact_strength: 0.04,
            enabled: true,
            gravity_enabled: true,
            mouse_repulsion_enabled: true,
            seed: 12345,
            current_shape: Arc::new(crate::shapes::Sphere::new()),
            material_density: 0.6, // Wood density by default
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
        
        let gravity = self.gravity;
        let radius = self.radius;
        let pool_width = self.pool_width;
        let pool_length = self.pool_length;
        let pool_depth = self.pool_depth;
        let pool_shape = self.pool_shape;
        let enabled = self.enabled;
        let gravity_enabled = self.gravity_enabled;
        let mouse_repulsion_enabled = self.mouse_repulsion_enabled;
        let seed = &mut self.seed;
        let material_density = self.material_density;
        
        // Clone arc to avoid borrow issues in loop
        let current_shape = self.current_shape.clone();

        self.ripples.clear();
        
        for (i, obj) in self.objects.iter_mut().enumerate() {
            obj.old_position = obj.position;
            
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

            // Buoyancy
            let submerged_depth = (water_height + radius - obj.position.y).max(0.0);
            let percent_underwater = (submerged_depth / (2.0 * radius)).min(1.0);
            
            if percent_underwater > 0.0 {
                let buoyancy_force = -gravity * percent_underwater;
                force += buoyancy_force;
                
                let damping = if material_density > 1.0 { 4.0 } else { 2.0 };
                force -= obj.velocity * (percent_underwater * damping);
                
                let rx = (Self::random_f32_static(seed) - 0.5) * 2.0;
                let ry = (Self::random_f32_static(seed) - 0.5) * 2.0;
                let rz = (Self::random_f32_static(seed) - 0.5) * 2.0;
                
                if material_density <= 1.0 {
                    let random_torque = Vec3::new(rx, ry, rz).normalize_or_zero() * 1.0 * percent_underwater;
                    obj.angular_velocity += random_torque * dt;
                }
            }

            // Mouse repulsion
            if mouse_repulsion_enabled {
                if let Some(mouse) = mouse_point {
                    let mut dist_vec = obj.position - mouse;
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
            
            // Drag
            if obj.velocity.length_squared() > 0.001 {
                let speed = obj.velocity.length();
                let drag_coeff = 0.5 + percent_underwater * 2.0;
                let drag_force = -obj.velocity.normalize() * (speed * speed * drag_coeff);
                force += drag_force;
                
                let roll_axis = Vec3::Y.cross(obj.velocity).normalize_or_zero();
                let roll_mult = if material_density > 1.0 { 0.2 } else { 1.0 };
                obj.angular_velocity += roll_axis * speed * roll_mult * dt;
            }
            
            // Integrate
            obj.velocity += force * dt;
            obj.position += obj.velocity * dt;

            // Ripple Generation
            let was_submerged = (obj.old_position.y - radius) < water_height;
            let is_submerged = (obj.position.y - radius) < water_height;
            
            if !was_submerged && is_submerged && obj.velocity.y < -0.1 {
                 self.ripples.push(RippleEvent {
                     x: obj.position.x,
                     z: obj.position.z,
                     strength: -0.05 * obj.velocity.y.abs().min(5.0),
                     radius: radius * 1.5,
                 });
            }

            if material_density > 1.0 && percent_underwater > 0.0 && percent_underwater < 0.9 {
                 if Self::random_f32_static(seed) < 0.08 {
                     self.ripples.push(RippleEvent {
                         x: obj.position.x + (Self::random_f32_static(seed) - 0.5) * radius * 0.5,
                         z: obj.position.z + (Self::random_f32_static(seed) - 0.5) * radius * 0.5,
                         strength: -0.015,
                         radius: radius * 0.4,
                     });
                 }
            }

            obj.velocity *= 0.995;
            obj.angular_velocity *= 0.98;
            
            let angle = obj.angular_velocity.length();
            if angle > 0.0001 {
                let axis = obj.angular_velocity / angle;
                let rot_delta = Quat::from_axis_angle(axis, angle * dt);
                obj.rotation = (rot_delta * obj.rotation).normalize();
            }
            
            // Wall collisions
            let collider = current_shape.as_ref().collider();
            let shape_params = ShapeParams {
                radius: self.radius,
                rotation: obj.rotation,
                scale: Vec3::splat(self.radius),
            };

            // Direct object modification (no adapter needed)
            match pool_shape {
                PoolShapeType::Tube => {
                    collider.collide_with_cylinder(
                        obj,
                        pool_width / 2.0,
                        current_shape.as_ref(),
                        &shape_params,
                    );
                },
                PoolShapeType::Frustum => {
                    collider.collide_with_frustum(
                        obj,
                        1.0,
                        0.7,
                        -pool_depth,
                        self.wall_height,
                        current_shape.as_ref(),
                        &shape_params,
                    );
                },
                _ => {
                    let half_width = pool_width / 2.0;
                    let half_length = pool_length / 2.0;
                    collider.collide_with_box(
                        obj,
                        half_width,
                        half_length,
                        current_shape.as_ref(),
                        &shape_params,
                    );
                }
            }
            
            // Floor collision
            let floor_y = -pool_depth;
            collider.collide_with_floor(
                obj,
                floor_y,
                current_shape.as_ref(),
                &shape_params,
                dt,
            );
        }

        self.solve_object_collisions();
    }

    fn solve_object_collisions(&mut self) {
        let count = self.objects.len();
        if count < 2 { return; }
        
        let collider = self.current_shape.as_ref().collider();
        let current_shape = self.current_shape.as_ref();
        
        const SOLVER_ITERATIONS: usize = 8;
        const POSITION_SLOP: f32 = 0.001;
        
        for _iter in 0..SOLVER_ITERATIONS {
            let mut corrections: Vec<(usize, Vec3)> = Vec::new();
            
            for i in 0..count {
                for j in (i + 1)..count {
                    // We can pass objects directly if check_object_collision takes PhysicsState
                    // But here we need to borrow immutably from self.objects while iterating
                    // Indexing solves this
                    
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

                    if let Some(collision) = collider.check_object_collision(
                        &self.objects[i],
                        &self.objects[j],
                        current_shape,
                        current_shape,
                        &shape_params_a,
                        &shape_params_b,
                    ) {
                        let penetration = collision.penetration_depth;
                        if penetration > POSITION_SLOP {
                            let normal = collision.normal;
                            let baumgarte = 0.2;
                            let correction_mag = (penetration - POSITION_SLOP) * baumgarte;
                            let correction = normal * correction_mag;
                            corrections.push((i, -correction));
                            corrections.push((j, correction));
                        }
                    }
                }
            }
            
            let mut accumulated: Vec<(Vec3, u32)> = vec![(Vec3::ZERO, 0); count];
            for (idx, corr) in corrections {
                accumulated[idx].0 += corr;
                accumulated[idx].1 += 1;
            }
            for (i, (total_corr, num)) in accumulated.into_iter().enumerate() {
                if num > 0 {
                    self.objects[i].position += total_corr / num as f32;
                }
            }
        }
        
        for i in 0..count {
            for j in (i + 1)..count {
                // Similar for velocity response
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

                if let Some(collision) = collider.check_object_collision(
                    &self.objects[i],
                    &self.objects[j],
                    current_shape,
                    current_shape,
                    &shape_params_a,
                    &shape_params_b,
                ) {
                    let normal = collision.normal;
                    let v1 = self.objects[i].velocity;
                    let v2 = self.objects[j].velocity;
                    let relative_vel = v1 - v2;
                    let closing_speed = relative_vel.dot(normal);
                    
                    if closing_speed < -0.001 {
                        let restitution = 0.1;
                        let impulse_mag = -(1.0 + restitution) * closing_speed * 0.5;
                        let impulse = normal * impulse_mag;
                        
                        self.objects[i].velocity += impulse;
                        self.objects[j].velocity -= impulse;
                        
                        let tangent_vel = relative_vel - normal * closing_speed;
                        if tangent_vel.length_squared() > 1e-6 {
                            let friction_coeff = 0.4;
                            let max_friction = impulse_mag * friction_coeff;
                            let friction_impulse = tangent_vel.normalize() * tangent_vel.length().min(max_friction);
                            
                            self.objects[i].velocity -= friction_impulse * 0.5;
                            self.objects[j].velocity += friction_impulse * 0.5;
                            
                            let torque_axis = normal.cross(tangent_vel).normalize_or_zero();
                            let torque = torque_axis * tangent_vel.length() * 0.3;
                            self.objects[i].angular_velocity += torque;
                            self.objects[j].angular_velocity -= torque;
                        }
                    } else {
                        let contact_damping = 0.92;
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
    }
    
    pub fn move_by(&mut self, index: usize, delta: Vec3) {
        if index >= self.objects.len() { return; }
        
        let obj = &mut self.objects[index];
        obj.position += delta;
        
        let current_shape = self.current_shape.clone();
        let collider = current_shape.as_ref().collider();
        
        let shape_params = ShapeParams {
            radius: self.radius,
            rotation: obj.rotation,
            scale: Vec3::splat(self.radius),
        };
        
        match self.pool_shape {
            PoolShapeType::Tube => {
                collider.collide_with_cylinder(
                    obj,
                    self.pool_width / 2.0,
                    current_shape.as_ref(),
                    &shape_params,
                );
            },
            PoolShapeType::Frustum => {
                collider.collide_with_frustum(
                    obj,
                    1.0, 
                    0.7, 
                    -self.pool_depth,
                    self.wall_height,
                    current_shape.as_ref(), 
                    &shape_params,
                );
            },
            _ => {
                let half_width = self.pool_width / 2.0;
                let half_length = self.pool_length / 2.0;
                collider.collide_with_box(
                    obj,
                    half_width,
                    half_length,
                    current_shape.as_ref(),
                    &shape_params,
                );
            }
        }
        
        obj.position.y = obj.position.y.clamp(
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
                position: Vec3::new(offset_x, 1.5 + (i as f32) * 0.2, offset_z),
                old_position: Vec3::new(offset_x, 1.5 + (i as f32) * 0.2, offset_z),
                ..Default::default()
            });
        }
    }
}
