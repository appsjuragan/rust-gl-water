//! Shared enumerations for the application

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeType {
    Sphere,
    Torus,
    Tetrahedron,
    Cube,
}

impl ShapeType {
    pub fn to_shader_int(&self) -> i32 {
        match self {
            Self::Sphere => 0,
            Self::Torus => 1,
            Self::Tetrahedron => 2,
            Self::Cube => 3,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sphere => "Sphere",
            Self::Torus => "Torus",
            Self::Tetrahedron => "Tetrahedron",
            Self::Cube => "Cube",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureType {
    Glass,
    Wood,
    Steel,
    Ice,
}

impl TextureType {
    pub fn to_shader_int(&self) -> i32 {
        match self {
            Self::Glass => 0,
            Self::Wood => 1,
            Self::Steel => 2,
            Self::Ice => 3,
        }
    }

    /// Returns material density relative to water (water = 1.0)
    pub fn density(&self) -> f32 {
        match self {
            Self::Glass => 2.5,   // Sinks
            Self::Wood => 0.6,    // Floats
            Self::Steel => 7.8,   // Sinks fast
            Self::Ice => 0.92,    // Floats barely
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolShapeType {
    Tube,
    Cube,
    Cuboid,
    Frustum,
}

impl PoolShapeType {
    pub fn to_shader_int(&self) -> i32 {
        match self {
            Self::Cube | Self::Cuboid => 0,
            Self::Frustum => 1,
            Self::Tube => 2,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tube => "Tube",
            Self::Cube => "Cube",
            Self::Cuboid => "Cuboid",
            Self::Frustum => "Frustum",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    Auto,
    Vulkan,
    OpenGL,
    Dx11,
    Dx12,
}
