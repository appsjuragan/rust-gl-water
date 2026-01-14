//! Physics abstraction traits
//!
//! Defines the core physics interfaces for collision detection and buoyancy calculation

use glam::{Quat, Vec3};
use crate::core::shape::{Shape, ShapeParams};

/// State of a physics object
#[allow(dead_code)]
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

/// Information about a collision between two objects
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CollisionInfo {
    pub normal: Vec3,
    pub penetration_depth: f32,
    pub contact_point: Vec3,
}

/// Pool shape types for collision detection
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoolType {
    Box { half_width: f32, half_length: f32 },
    Cylinder { radius: f32 },
    Frustum { top_scale: f32, bottom_scale: f32 },
}

/// Trait for collision detection
#[allow(dead_code)]
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
                self.collide_with_frustum(state, top_scale, bottom_scale, shape, shape_params);
            }
        }
    }
}

/// Trait for buoyancy force calculation
#[allow(dead_code)]
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
        let velocity_magnitude = state.velocity.length();
        if velocity_magnitude < 1e-6 {
            return Vec3::ZERO;
        }
        let drag_strength = drag_coefficient * submerged_fraction * velocity_magnitude;
        -state.velocity.normalize() * drag_strength
    }
}
