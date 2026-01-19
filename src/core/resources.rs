//! Resource loading utilities
//!
//! This module provides utilities for loading external resources like shaders and textures

use std::path::Path;

/// Shader loader with caching
pub struct ShaderLoader {
    pub base_path: std::path::PathBuf,
}

impl ShaderLoader {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Load a shader from file or return embedded content
    pub fn load(&self, name: &str) -> Result<String, std::io::Error> {
        let path = self.base_path.join(name);
        
        // Try loading from file system
        if path.exists() {
            return std::fs::read_to_string(&path);
        }

        // Fallback to embedded shaders
        self.load_embedded(name)
    }

    /// Load embedded shader content
    fn load_embedded(&self, name: &str) -> Result<String, std::io::Error> {
        match name {
            "common.wgsl" => Ok(include_str!("../shaders/wgsl/common.wgsl").to_string()),
            "water_render.wgsl" => {
                let common = include_str!("../shaders/wgsl/common.wgsl");
                let water = include_str!("../shaders/wgsl/water_render.wgsl");
                Ok(format!("{}\n{}", common, water))
            }
            "pool.wgsl" => {
                let common = include_str!("../shaders/wgsl/common.wgsl");
                let pool = include_str!("../shaders/wgsl/pool.wgsl");
                Ok(format!("{}\n{}", common, pool))
            }
            "caustics.wgsl" => {
                let common = include_str!("../shaders/wgsl/common.wgsl");
                let caustics = include_str!("../shaders/wgsl/caustics.wgsl");
                Ok(format!("{}\n{}", common, caustics))
            }
            "sphere.wgsl" => {
                let common = include_str!("../shaders/wgsl/common.wgsl");
                let sphere = include_str!("../shaders/wgsl/sphere.wgsl");
                Ok(format!("{}\n{}", common, sphere))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Shader '{}' not found", name),
            )),
        }
    }
}

impl Default for ShaderLoader {
    fn default() -> Self {
        Self::new("assets/shaders")
    }
}

/// Texture loading utilities
pub struct TextureLoader;

impl TextureLoader {
    /// Load texture from file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<image::RgbaImage, image::ImageError> {
        let img = image::open(path)?;
        Ok(img.to_rgba8())
    }

    /// Create a solid color texture
    pub fn create_solid_color(color: [u8; 4], size: u32) -> image::RgbaImage {
        image::RgbaImage::from_pixel(size, size, image::Rgba(color))
    }

    /// Create a tile pattern texture
    pub fn create_tile_pattern(base_color: [u8; 4], size: u32, tile_size: u32) -> image::RgbaImage {
        let mut img = image::RgbaImage::new(size, size);
        
        for y in 0..size {
            for x in 0..size {
                let is_edge = x % tile_size < 2 || y % tile_size < 2;
                let color = if is_edge {
                    [
                        (base_color[0] as f32 * 0.7) as u8,
                        (base_color[1] as f32 * 0.7) as u8,
                        (base_color[2] as f32 * 0.7) as u8,
                        base_color[3],
                    ]
                } else {
                    base_color
                };
                img.put_pixel(x, y, image::Rgba(color));
            }
        }
        
        img
    }
}
