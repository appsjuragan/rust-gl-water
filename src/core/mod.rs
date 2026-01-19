//! Core abstractions module

pub mod shape;
pub mod physics_trait;
pub mod material;
pub mod graphics_backend;
pub mod constants;
pub mod enums;
pub mod geometry;
pub mod state;

pub use shape::{Shape, ShapeParams, MeshParams, ShapeRegistry, BoxedShape};
pub use physics_trait::{Collider, CollisionInfo, BuoyancyCalculator};
pub use material::{Material, MaterialType, MaterialProperties, TextureSource, TextureLoader, PoolTexture};
pub use graphics_backend::{GraphicsBackend, BackendType};
pub use constants::*;
pub use enums::*;
pub use geometry::*;
pub use state::*;
