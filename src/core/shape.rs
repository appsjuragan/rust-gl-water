//! Core shape abstraction trait
//!
//! This module defines the fundamental `Shape` trait that all geometric shapes must implement.
//! Shapes provide unified interface for rendering, physics, and shader code generation.

use glam::{Quat, Vec3};
use std::sync::Arc;

/// Parameters for shape SDF evaluation
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ShapeParams {
    /// Radius or characteristic size of shape
    pub radius: f32,
    /// Rotation quaternion
    pub rotation: Quat,
    /// Scale factors (currently uniform via radius)
    pub scale: Vec3,
}

impl Default for ShapeParams {
    fn default() -> Self {
        Self {
            radius: 1.0,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

/// Parameters for mesh generation
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MeshParams {
    /// Radius or characteristic size
    pub radius: f32,
    /// Number of subdivisions/segments
    pub subdivisions: u32,
    /// Additional subdivisions for secondary dimension (e.g., tubular segments for torus)
    pub subdivisions_secondary: Option<u32>,
}

impl Default for MeshParams {
    fn default() -> Self {
        Self {
            radius: 1.0,
            subdivisions: 32,
            subdivisions_secondary: None,
        }
    }
}

/// Vertex format for shape meshes
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// Core trait for all geometric shapes
#[allow(dead_code)]
pub trait Shape: Send + Sync {
    fn name(&self) -> &str;
    fn sdf(&self, point: Vec3, params: &ShapeParams) -> f32;
    fn generate_mesh(&self, params: &MeshParams) -> (Vec<ShapeVertex>, Vec<u32>);
    fn bounding_sphere(&self, params: &ShapeParams) -> (Vec3, f32);
    fn shader_intersection_code(&self) -> String;
    fn shader_sdf_code(&self) -> String;
    fn volume(&self, params: &ShapeParams) -> f32;
    fn submerged_volume(&self, params: &ShapeParams, center_y: f32, water_height: f32) -> f32;
    fn collider(&self) -> Box<dyn crate::core::physics_trait::Collider>;
}

/// Type alias for boxed shape trait object
pub type BoxedShape = Arc<dyn Shape>;

/// Shape registry for runtime shape lookup
#[allow(dead_code)]
pub struct ShapeRegistry {
    shapes: Vec<BoxedShape>,
}

#[allow(dead_code)]
impl ShapeRegistry {
    pub fn new() -> Self {
        Self { shapes: Vec::new() }
    }

    pub fn register(&mut self, shape: BoxedShape) {
        self.shapes.push(shape);
    }

    pub fn get(&self, name: &str) -> Option<BoxedShape> {
        self.shapes.iter().find(|s| s.name() == name).cloned()
    }

    pub fn shape_names(&self) -> Vec<String> {
        self.shapes.iter().map(|s| s.name().to_string()).collect()
    }

    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }
}

impl Default for ShapeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
