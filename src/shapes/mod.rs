//! Shape implementations

#![allow(dead_code)]

pub mod sphere;
pub mod cube;
pub mod torus;
pub mod tetrahedron;

pub use sphere::Sphere;
pub use cube::Cube;
pub use torus::Torus;
pub use tetrahedron::Tetrahedron;

use crate::core::shape::ShapeRegistry;
use std::sync::Arc;

pub fn create_default_registry() -> ShapeRegistry {
    let mut registry = ShapeRegistry::new();
    registry.register(Arc::new(Sphere::new()));
    registry.register(Arc::new(Cube::new()));
    registry.register(Arc::new(Torus::new()));
    registry.register(Arc::new(Tetrahedron::new()));
    registry
}
