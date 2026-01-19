//! Scene management abstraction
//!
//! This module defines the Scene struct which manages all objects, lights, and camera

use glam::{Mat4, Vec3};
use crate::camera::Camera;

/// Light representation
#[derive(Debug, Clone)]
pub struct Light {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.577, 0.577, 0.577).normalize(),
            color: Vec3::ONE,
            intensity: 1.0,
        }
    }
}

/// Transform component for objects
#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: glam::Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_rotation(mut self, rotation: glam::Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// Mesh data
#[derive(Debug, Clone)]
pub struct Mesh {
    pub name: String,
    pub vertex_count: usize,
    pub index_count: usize,
}

/// Object in the scene
#[derive(Debug, Clone)]
pub struct Object {
    pub name: String,
    pub transform: Transform,
    pub mesh: Option<Mesh>,
    pub material_id: Option<usize>,
}

impl Object {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transform: Transform::default(),
            mesh: None,
            material_id: None,
        }
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_mesh(mut self, mesh: Mesh) -> Self {
        self.mesh = Some(mesh);
        self
    }

    pub fn with_material(mut self, material_id: usize) -> Self {
        self.material_id = Some(material_id);
        self
    }
}

/// Scene contains all renderable objects
pub struct Scene {
    pub camera: Camera,
    pub lights: Vec<Light>,
    pub objects: Vec<Object>,
    pub ambient_color: Vec3,
}

impl Scene {
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            lights: vec![Light::default()],
            objects: Vec::new(),
            ambient_color: Vec3::new(0.1, 0.1, 0.1),
        }
    }

    pub fn add_object(&mut self, object: Object) -> usize {
        self.objects.push(object);
        self.objects.len() - 1
    }

    pub fn add_light(&mut self, light: Light) -> usize {
        self.lights.push(light);
        self.lights.len() - 1
    }

    pub fn get_object_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.objects.get_mut(index)
    }

    pub fn get_light_mut(&mut self, index: usize) -> Option<&mut Light> {
        self.lights.get_mut(index)
    }

    pub fn clear_objects(&mut self) {
        self.objects.clear();
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new(Camera::default())
    }
}
