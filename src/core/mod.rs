//! Core abstractions module

#![allow(unused_imports)]

pub mod shape;
pub mod physics_trait;
pub mod material;
pub mod graphics_backend;

pub use shape::{Shape, ShapeParams, MeshParams, ShapeVertex, ShapeRegistry, BoxedShape};
pub use physics_trait::{PhysicsState, Collider, CollisionInfo, BuoyancyCalculator, PoolType};
pub use material::{Material, MaterialType, MaterialProperties, TextureSource, TextureLoader, PoolTexture};
pub use graphics_backend::{GraphicsBackend, BackendType, BufferDesc, TextureDesc, ShaderSource};
