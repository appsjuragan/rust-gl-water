//! Standard collider implementation using bounding spheres

use crate::core::physics_trait::{Collider, CollisionInfo, PhysicsState};
use crate::core::shape::{Shape, ShapeParams};
use glam::Vec3;

/// Standard bounding-sphere based collider
#[allow(dead_code)]
pub struct BoundingSphereCollider;

impl BoundingSphereCollider {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

impl Default for BoundingSphereCollider {
    fn default() -> Self {
        Self::new()
    }
}

impl Collider for BoundingSphereCollider {
    fn collide_with_box(
        &self,
        state: &mut PhysicsState,
        half_width: f32,
        half_length: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let (_, radius) = shape.bounding_sphere(shape_params);

        // X boundaries
        if state.position.x < -half_width + radius {
            state.position.x = -half_width + radius;
            if state.velocity.x < 0.0 {
                state.velocity.x = -state.velocity.x * 0.5;
            }
        } else if state.position.x > half_width - radius {
            state.position.x = half_width - radius;
            if state.velocity.x > 0.0 {
                state.velocity.x = -state.velocity.x * 0.5;
            }
        }

        // Z boundaries
        if state.position.z < -half_length + radius {
            state.position.z = -half_length + radius;
            if state.velocity.z < 0.0 {
                state.velocity.z = -state.velocity.z * 0.5;
            }
        } else if state.position.z > half_length - radius {
            state.position.z = half_length - radius;
            if state.velocity.z > 0.0 {
                state.velocity.z = -state.velocity.z * 0.5;
            }
        }
    }

    fn collide_with_cylinder(
        &self,
        state: &mut PhysicsState,
        radius: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let (_, obj_radius) = shape.bounding_sphere(shape_params);
        let max_radius = radius - obj_radius;

        let dist_xz = (state.position.x * state.position.x + state.position.z * state.position.z).sqrt();

        if dist_xz > max_radius {
            let normal = Vec3::new(state.position.x, 0.0, state.position.z).normalize();
            state.position = normal * max_radius + Vec3::new(0.0, state.position.y, 0.0);

            let vel_normal = state.velocity.dot(normal);
            if vel_normal > 0.0 {
                state.velocity -= normal * vel_normal * 1.5;
            }
        }
    }

    fn collide_with_frustum(
        &self,
        state: &mut PhysicsState,
        top_scale: f32,
        bottom_scale: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let avg_scale = (top_scale + bottom_scale) / 2.0;
        self.collide_with_box(state, avg_scale, avg_scale, shape, shape_params);
    }

    fn check_object_collision(
        &self,
        state_a: &PhysicsState,
        state_b: &PhysicsState,
        shape_a: &dyn Shape,
        shape_b: &dyn Shape,
        params_a: &ShapeParams,
        params_b: &ShapeParams,
    ) -> Option<CollisionInfo> {
        let (_, radius_a) = shape_a.bounding_sphere(params_a);
        let (_, radius_b) = shape_b.bounding_sphere(params_b);

        let delta = state_b.position - state_a.position;
        let dist = delta.length();
        let min_dist = radius_a + radius_b;

        if dist < min_dist && dist > 1e-6 {
            Some(CollisionInfo {
                normal: delta / dist,
                penetration_depth: min_dist - dist,
                contact_point: state_a.position + delta * (radius_a / min_dist),
            })
        } else {
            None
        }
    }
}

/// Default buoyancy calculator implementation
#[allow(dead_code)]
pub struct StandardBuoyancy;

impl StandardBuoyancy {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

impl Default for StandardBuoyancy {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::core::physics_trait::BuoyancyCalculator for StandardBuoyancy {}
