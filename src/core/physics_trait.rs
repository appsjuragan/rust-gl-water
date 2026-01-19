//! Physics abstraction traits

#![allow(dead_code)]

use glam::{Quat, Vec3};
use crate::core::shape::{Shape, ShapeParams};

#[derive(Clone, Copy, Debug)]
pub struct PhysicsState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub angular_velocity: Vec3,
    pub mass: f32,
}

impl Default for PhysicsState {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            mass: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CollisionInfo {
    pub normal: Vec3,
    pub penetration_depth: f32,
    pub contact_point: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoolType {
    Box { half_width: f32, half_length: f32 },
    Cylinder { radius: f32 },
    Frustum { top_scale: f32, bottom_scale: f32 },
}

pub trait Collider: Send + Sync {
    fn collide_with_box(
        &self,
        state: &mut PhysicsState,
        half_width: f32,
        half_length: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    );

    fn collide_with_cylinder(
        &self,
        state: &mut PhysicsState,
        radius: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    );

    fn collide_with_frustum(
        &self,
        state: &mut PhysicsState,
        top_scale: f32,
        bottom_scale: f32,
        y_min: f32,
        y_max: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    );

    fn check_object_collision(
        &self,
        state_a: &PhysicsState,
        state_b: &PhysicsState,
        shape_a: &dyn Shape,
        shape_b: &dyn Shape,
        params_a: &ShapeParams,
        params_b: &ShapeParams,
    ) -> Option<CollisionInfo>;

    fn collide_with_pool(
        &self,
        state: &mut PhysicsState,
        pool_type: PoolType,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        match pool_type {
            PoolType::Box { half_width, half_length } => {
                self.collide_with_box(state, half_width, half_length, shape, shape_params);
            }
            PoolType::Cylinder { radius } => {
                self.collide_with_cylinder(state, radius, shape, shape_params);
            }
            PoolType::Frustum { top_scale, bottom_scale } => {
                // Default implementation assumes standard pool depth/height if not provided in PoolType
                // But PoolType::Frustum doesn't have y_min/y_max.
                // We'll assume -1.0 to 0.4 for now in this default impl, but implementations should override or we should update PoolType.
                self.collide_with_frustum(state, top_scale, bottom_scale, -1.0, 0.4, shape, shape_params);
            }
        }
    }
    
    /// Collide with floor at given y level. Returns true if contact was made.
    fn collide_with_floor(
        &self,
        state: &mut PhysicsState,
        floor_y: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
        dt: f32,
    ) -> bool {
        let (_, radius) = shape.bounding_sphere(shape_params);
        let min_y = floor_y + radius;
        
        if state.position.y < min_y {
            state.position.y = min_y;
            
            // Bounce only if falling fast
            if state.velocity.y < -0.05 {
                state.velocity.y = -state.velocity.y * 0.3;
            } else {
                state.velocity.y = 0.0;
            }
            
            // Friction
            let v_horiz = Vec3::new(state.velocity.x, 0.0, state.velocity.z);
            let horiz_speed = v_horiz.length();
            if horiz_speed > 0.01 {
                let friction = 0.4;
                let friction_decel = friction * 9.81 * dt;
                if horiz_speed < friction_decel {
                    state.velocity.x = 0.0;
                    state.velocity.z = 0.0;
                } else {
                    let friction_dir = -v_horiz.normalize();
                    state.velocity += friction_dir * friction_decel;
                }
            } else {
                state.velocity.x = 0.0;
                state.velocity.z = 0.0;
            }
            
            // Angular damping
            state.angular_velocity *= 0.9;
            
            true
        } else {
            false
        }
    }
}

pub trait BuoyancyCalculator: Send + Sync {
    fn calculate_buoyancy(
        &self,
        state: &PhysicsState,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
        water_height: f32,
        water_density: f32,
        gravity: f32,
    ) -> Vec3 {
        let submerged_volume = shape.submerged_volume(shape_params, state.position.y, water_height);
        Vec3::new(0.0, water_density * submerged_volume * gravity, 0.0)
    }

    fn calculate_drag(
        &self,
        state: &PhysicsState,
        submerged_fraction: f32,
        drag_coefficient: f32,
    ) -> Vec3 {
        let speed = state.velocity.length();
        if speed < 1e-6 { return Vec3::ZERO; }
        -state.velocity.normalize() * drag_coefficient * submerged_fraction * speed
    }
}
