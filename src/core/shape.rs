//! Core shape abstraction trait

#![allow(dead_code)]

use glam::{Quat, Vec3};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ShapeParams {
    pub radius: f32,
    pub rotation: Quat,
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

#[derive(Clone, Debug)]
pub struct MeshParams {
    pub radius: f32,
    pub subdivisions: u32,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

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

pub type BoxedShape = Arc<dyn Shape>;

pub struct ShapeRegistry {
    shapes: Vec<BoxedShape>,
}

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

    pub fn len(&self) -> usize { self.shapes.len() }
    pub fn is_empty(&self) -> bool { self.shapes.is_empty() }
}

impl Default for ShapeRegistry {
    fn default() -> Self { Self::new() }
}
