//! Graphics backend abstraction

#![allow(dead_code)]

use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    WGPU, OpenGL, Vulkan, DirectX11, DirectX12,
}

impl BackendType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::WGPU => "WebGPU (wgpu)",
            Self::OpenGL => "OpenGL",
            Self::Vulkan => "Vulkan",
            Self::DirectX11 => "DirectX 11",
            Self::DirectX12 => "DirectX 12",
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::WGPU => true,
            Self::OpenGL | Self::Vulkan => cfg!(not(target_arch = "wasm32")),
            Self::DirectX11 | Self::DirectX12 => cfg!(target_os = "windows"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BufferUsage {
    pub vertex: bool,
    pub index: bool,
    pub uniform: bool,
    pub storage: bool,
    pub copy_src: bool,
    pub copy_dst: bool,
}

impl BufferUsage {
    pub const VERTEX: Self = Self { vertex: true, index: false, uniform: false, storage: false, copy_src: false, copy_dst: true };
    pub const INDEX: Self = Self { vertex: false, index: true, uniform: false, storage: false, copy_src: false, copy_dst: true };
    pub const UNIFORM: Self = Self { vertex: false, index: false, uniform: true, storage: false, copy_src: false, copy_dst: true };
}

pub struct BufferDesc {
    pub label: String,
    pub size: u64,
    pub usage: BufferUsage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormatType {
    RGBA8Unorm, RGBA8Srgb, BGRA8Unorm, R32Float, RGBA32Float, Depth32Float,
}

pub struct TextureDesc {
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormatType,
    pub mip_levels: u32,
    pub usage_texture: bool,
    pub usage_render_target: bool,
    pub usage_storage: bool,
}

pub enum ShaderSource {
    WGSL(String),
    GLSL { vertex: String, fragment: String },
    HLSL { vertex: String, fragment: String },
    SPIRV(Vec<u32>),
}

pub trait GpuBuffer: Send + Sync {
    fn write(&mut self, data: &[u8], offset: u64);
    fn size(&self) -> u64;
}

pub trait GpuTexture: Send + Sync {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn write(&mut self, data: &[u8], width: u32, height: u32, x: u32, y: u32);
}

pub trait Shader: Send + Sync {
    fn source(&self) -> &str;
}

pub trait RenderPipeline: Send + Sync {
    fn label(&self) -> &str;
}

pub struct PipelineDesc {
    pub label: String,
    pub vertex_shader: Arc<dyn Shader>,
    pub fragment_shader: Arc<dyn Shader>,
    pub vertex_buffer_layout: Vec<VertexAttributeDesc>,
}

pub struct VertexAttributeDesc {
    pub format: VertexFormat,
    pub offset: u64,
    pub location: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    Float32x2, Float32x3, Float32x4,
}

pub trait FrameContext: Send + Sync {
    fn submit(&mut self);
}

pub trait GraphicsBackend: Send + Sync {
    fn backend_type(&self) -> BackendType;
    fn backend_name(&self) -> &str { self.backend_type().name() }
    fn create_buffer(&self, desc: &BufferDesc) -> Result<Box<dyn GpuBuffer>, String>;
    fn create_texture(&self, desc: &TextureDesc) -> Result<Box<dyn GpuTexture>, String>;
    fn create_shader(&self, source: &ShaderSource) -> Result<Arc<dyn Shader>, String>;
    fn create_pipeline(&self, desc: &PipelineDesc) -> Result<Box<dyn RenderPipeline>, String>;
    fn begin_frame(&mut self) -> Result<Box<dyn FrameContext>, String>;
}

pub fn create_backend(backend_type: BackendType) -> Result<Box<dyn GraphicsBackend>, String> {
    if !backend_type.is_available() {
        return Err(format!("{} is not available on this platform", backend_type.name()));
    }
    Err(format!("{} backend not yet implemented", backend_type.name()))
}
