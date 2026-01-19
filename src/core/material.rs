//! Material and texture abstraction

#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialType {
    Glass,
    Wood,
    Steel,
    Ice,
    Custom,
}

impl MaterialType {
    pub fn properties(&self) -> MaterialProperties {
        match self {
            Self::Glass => MaterialProperties {
                ior: 1.5, base_color: [0.9, 0.95, 1.0], absorption: [0.05, 0.02, 0.0],
                specularity: 1.0, roughness: 0.0,
            },
            Self::Wood => MaterialProperties {
                ior: 1.3, base_color: [0.4, 0.25, 0.1], absorption: [1.0, 1.0, 1.0],
                specularity: 0.1, roughness: 0.8,
            },
            Self::Steel => MaterialProperties {
                ior: 2.5, base_color: [0.8, 0.82, 0.85], absorption: [1.0, 1.0, 1.0],
                specularity: 2.0, roughness: 0.1,
            },
            Self::Ice => MaterialProperties {
                ior: 1.31, base_color: [0.85, 0.95, 1.0], absorption: [0.1, 0.05, 0.0],
                specularity: 0.8, roughness: 0.2,
            },
            Self::Custom => MaterialProperties::default(),
        }
    }

    pub fn shader_index(&self) -> i32 {
        match self {
            Self::Glass => 0, Self::Wood => 1, Self::Steel => 2, Self::Ice => 3, Self::Custom => 4,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TextureSource {
    Builtin(String),
    File(String),
    RawData { width: u32, height: u32, data: Arc<Vec<u8>> },
}

#[derive(Clone, Debug)]
pub struct Material {
    pub material_type: MaterialType,
    pub properties: MaterialProperties,
    pub texture: Option<TextureSource>,
}

impl Material {
    pub fn from_type(material_type: MaterialType) -> Self {
        Self { material_type, properties: material_type.properties(), texture: None }
    }

    pub fn with_texture(material_type: MaterialType, texture: TextureSource) -> Self {
        Self { material_type, properties: material_type.properties(), texture: Some(texture) }
    }

    pub fn custom(properties: MaterialProperties, texture: Option<TextureSource>) -> Self {
        Self { material_type: MaterialType::Custom, properties, texture }
    }
}

impl Default for Material {
    fn default() -> Self { Self::from_type(MaterialType::Glass) }
}

pub trait TextureLoader: Send + Sync {
    fn load_from_file(&self, path: &Path) -> Result<TextureSource, String>;
    fn create_from_data(&self, width: u32, height: u32, data: Vec<u8>) -> TextureSource {
        TextureSource::RawData { width, height, data: Arc::new(data) }
    }
    fn generate_procedural(&self, name: &str, size: u32) -> Result<TextureSource, String>;
}

#[derive(Clone, Debug)]
pub struct PoolTexture {
    pub floor: TextureSource,
    pub wall: Option<TextureSource>,
}

impl Default for PoolTexture {
    fn default() -> Self {
        Self { floor: TextureSource::Builtin("tiles".to_string()), wall: None }
    }
}

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
