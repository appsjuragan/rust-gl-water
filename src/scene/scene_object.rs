//! Scene object representation combining shape, physics, and rendering

use crate::core::shape::{BoxedShape, ShapeParams};
use crate::core::physics_trait::{PhysicsState, Collider, BuoyancyCalculator};
use crate::core::material::Material;

/// Unique identifier for scene objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(pub usize);

/// Transform component for scene objects
#[derive(Clone, Debug)]
pub struct Transform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: 1.0,
        }
    }
}

/// Unified scene object combining all aspects
#[allow(dead_code)]
pub struct SceneObject {
    pub id: ObjectId,
    pub shape: BoxedShape,
    pub transform: Transform,
    pub physics: PhysicsState,
    pub material: Material,
    pub collider: Box<dyn Collider>,
    pub buoyancy_calc: Box<dyn BuoyancyCalculator>,
}

impl SceneObject {
    /// Create a new scene object
    #[allow(dead_code)]
    pub fn new(
        id: ObjectId,
        shape: BoxedShape,
        material: Material,
        collider: Box<dyn Collider>,
        buoyancy_calc: Box<dyn BuoyancyCalculator>,
    ) -> Self {
        Self {
            id,
            shape,
            transform: Transform::default(),
            physics: PhysicsState::default(),
            material,
            collider,
            buoyancy_calc,
        }
    }

    /// Get shape parameters from transform
    pub fn shape_params(&self) -> ShapeParams {
        ShapeParams {
            radius: self.transform.scale,
            rotation: self.transform.rotation,
            scale: glam::Vec3::splat(self.transform.scale),
        }
    }

    /// Set position
    #[allow(dead_code)]
    pub fn set_position(&mut self, position: glam::Vec3) {
        self.transform.position = position;
        self.physics.position = position;
    }

    /// Apply physics integration
    #[allow(dead_code)]
    pub fn integrate_physics(&mut self, dt: f32, gravity: f32, water_height: f32, water_density: f32) {
        let gravity_force = glam::Vec3::new(0.0, -gravity * self.physics.mass, 0.0);

        let buoyancy_force = self.buoyancy_calc.calculate_buoyancy(
            &self.physics,
            self.shape.as_ref(),
            &self.shape_params(),
            water_height,
            water_density,
            gravity,
        );

        let submerged_fraction = if self.transform.position.y < water_height { 1.0 } else { 0.0 };
        let drag_force = self.buoyancy_calc.calculate_drag(&self.physics, submerged_fraction, 0.5);

        let total_force = gravity_force + buoyancy_force + drag_force;

        self.physics.velocity += (total_force / self.physics.mass) * dt;
        self.physics.velocity *= 0.99;
        self.physics.position += self.physics.velocity * dt;

        self.transform.position = self.physics.position;
    }

    /// Collide with pool
    #[allow(dead_code)]
    pub fn collide_with_pool(&mut self, pool_type: crate::core::physics_trait::PoolType) {
        let params = self.shape_params();
        let shape_ref = self.shape.clone();
        
        self.collider.collide_with_pool(
            &mut self.physics,
            pool_type,
            shape_ref.as_ref(),
            &params,
        );
        self.transform.position = self.physics.position;
    }

    /// Check collision with another object
    #[allow(dead_code)]
    pub fn check_collision(&self, other: &SceneObject) -> Option<crate::core::physics_trait::CollisionInfo> {
        let self_params = self.shape_params();
        let other_params = other.shape_params();
        
        self.collider.check_object_collision(
            &self.physics,
            &other.physics,
            self.shape.as_ref(),
            other.shape.as_ref(),
            &self_params,
            &other_params,
        )
    }
}
