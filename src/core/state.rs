//! Unified physics state definition

use glam::{Vec3, Quat};

/// Represents the physical state of a rigid body
#[derive(Clone, Copy, Debug)]
pub struct RigidBody {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub angular_velocity: Vec3,
    pub mass: f32,
    
    // Additional fields previously in ObjectState but not PhysicsState
    pub old_position: Vec3, // For simulation integration (Verlet/PBD)
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            mass: 1.0,
            old_position: Vec3::ZERO,
        }
    }
}

impl RigidBody {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            old_position: position,
            ..Default::default()
        }
    }
}
