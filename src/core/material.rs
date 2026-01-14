//! Material and texture abstraction
//!
//! Defines materials and texture loading for objects and pools with support for user-provided images

use std::path::Path;
use std::sync::Arc;

/// Material properties for rendering
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MaterialProperties {
    pub ior: f32,
    pub base_color: [f32; 3],
    pub absorption: [f32; 3],
    pub specularity: f32,
    pub roughness: f32,
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            ior: 1.5,
            base_color: [0.9, 0.95, 1.0],
            absorption: [0.05, 0.02, 0.0],
            specularity: 1.0,
            roughness: 0.0,
        }
    }
}

/// Predefined material types
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialType {
    Glass,
    Wood,
    Steel,
    Ice,
    Custom,
}

#[allow(dead_code)]
impl MaterialType {
    pub fn properties(&self) -> MaterialProperties {
        match self {
            MaterialType::Glass => MaterialProperties {
                ior: 1.5,
                base_color: [0.9, 0.95, 1.0],
                absorption: [0.05, 0.02, 0.0],
                specularity: 1.0,
                roughness: 0.0,
            },
            MaterialType::Wood => MaterialProperties {
                ior: 1.3,
                base_color: [0.4, 0.25, 0.1],
                absorption: [1.0, 1.0, 1.0],
                specularity: 0.1,
                roughness: 0.8,
            },
            MaterialType::Steel => MaterialProperties {
                ior: 2.5,
                base_color: [0.8, 0.82, 0.85],
                absorption: [1.0, 1.0, 1.0],
                specularity: 2.0,
                roughness: 0.1,
            },
            MaterialType::Ice => MaterialProperties {
                ior: 1.31,
                base_color: [0.85, 0.95, 1.0],
                absorption: [0.1, 0.05, 0.0],
                specularity: 0.8,
                roughness: 0.2,
            },
            MaterialType::Custom => MaterialProperties::default(),
        }
    }

    pub fn shader_index(&self) -> i32 {
        match self {
            MaterialType::Glass => 0,
            MaterialType::Wood => 1,
            MaterialType::Steel => 2,
            MaterialType::Ice => 3,
            MaterialType::Custom => 4,
        }
    }
}

/// Texture source - either built-in or user-provided
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum TextureSource {
    Builtin(String),
    File(String),
    RawData { width: u32, height: u32, data: Arc<Vec<u8>> },
}

/// Material definition combining properties and optional texture
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Material {
    pub material_type: MaterialType,
    pub properties: MaterialProperties,
    pub texture: Option<TextureSource>,
}

#[allow(dead_code)]
impl Material {
    pub fn from_type(material_type: MaterialType) -> Self {
        Self {
            material_type,
            properties: material_type.properties(),
            texture: None,
        }
    }

    pub fn with_texture(material_type: MaterialType, texture: TextureSource) -> Self {
        Self {
            material_type,
            properties: material_type.properties(),
            texture: Some(texture),
        }
    }

    pub fn custom(properties: MaterialProperties, texture: Option<TextureSource>) -> Self {
        Self {
            material_type: MaterialType::Custom,
            properties,
            texture,
        }
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::from_type(MaterialType::Glass)
    }
}

/// Trait for loading textures from various sources
#[allow(dead_code)]
pub trait TextureLoader: Send + Sync {
    fn load_from_file(&self, path: &Path) -> Result<TextureSource, String>;

    fn create_from_data(&self, width: u32, height: u32, data: Vec<u8>) -> TextureSource {
        TextureSource::RawData {
            width,
            height,
            data: Arc::new(data),
        }
    }

    fn generate_procedural(&self, name: &str, size: u32) -> Result<TextureSource, String>;
}

/// Pool texture configuration with support for user textures
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PoolTexture {
    pub floor: TextureSource,
    pub wall: Option<TextureSource>,
}

impl Default for PoolTexture {
    fn default() -> Self {
        Self {
            floor: TextureSource::Builtin("tiles".to_string()),
            wall: None,
        }
    }
}

#[allow(dead_code)]
impl PoolTexture {
    pub fn from_files(floor_path: &Path, wall_path: Option<&Path>) -> Self {
        Self {
            floor: TextureSource::File(floor_path.to_string_lossy().to_string()),
            wall: wall_path.map(|p| TextureSource::File(p.to_string_lossy().to_string())),
        }
    }

    pub fn wall_texture(&self) -> &TextureSource {
        self.wall.as_ref().unwrap_or(&self.floor)
    }
}
