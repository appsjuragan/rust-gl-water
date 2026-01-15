//! Core abstractions module
//!
//! This module contains the fundamental trait definitions and abstractions
//! that enable portability and extensibility throughout the application.

pub mod shape;
pub mod physics_trait;
pub mod material;
pub mod graphics_backend;
pub mod scene;
pub mod resources;

// Re-exports for convenience (will be used during integration)
#[allow(unused_imports)]
pub use shape::{Shape, ShapeParams, MeshParams, ShapeVertex, ShapeRegistry, BoxedShape};
#[allow(unused_imports)]
pub use physics_trait::{PhysicsState, Collider, CollisionInfo, BuoyancyCalculator, PoolType};
#[allow(unused_imports)]
pub use material::{Material, MaterialType, MaterialProperties, TextureSource, TextureLoader, PoolTexture};
#[allow(unused_imports)]
pub use graphics_backend::{GraphicsBackend, BackendType, BufferDesc, TextureDesc, ShaderSource};
#[allow(unused_imports)]
pub use scene::{Scene, Object, Transform, Light, Mesh};
#[allow(unused_imports)]
pub use resources::{ShaderLoader, TextureLoader as ResourceTextureLoader};
