//! Sphere shader - renders the floating sphere

use super::common::COMMON_UNIFORMS;

pub fn sphere_shader() -> String {
    format!(
        r#"
{COMMON_UNIFORMS}

struct CameraUniforms {{
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye: vec4<f32>,
}}

struct VertexInput {{
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}}

struct VertexOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) view_pos: vec3<f32>,
}}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> uniforms: CommonUniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {{
    var out: VertexOutput;
    
    // Transform unit sphere to world position
    let world_pos = in.position * uniforms.sphere_radius + uniforms.sphere_center.xyz;
    out.world_pos = world_pos;
    out.world_normal = in.normal;
    
    out.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.view_pos = camera.eye.xyz;
    
    return out;
}}

@group(1) @binding(0) var water_texture: texture_2d<f32>;
@group(1) @binding(1) var water_sampler: sampler;
// Bindings 2,3 are Tile.
@group(1) @binding(4) var caustic_texture: texture_2d<f32>;
@group(1) @binding(5) var caustic_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let light_dir = normalize(-uniforms.light_dir.xyz);
    let normal = normalize(in.world_normal);
    let view_dir = normalize(in.view_pos - in.world_pos);
    
    // Basic Phong lighting
    let ambient = 0.3;
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    // Specular
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    
    // Sample caustics
    // For simplicity, we'll skip mapping proper caustics to sphere for now
    
    let object_color = vec3<f32>(0.9, 0.95, 1.0); // Sphere color
    
    let color = object_color * (ambient + diffuse * 0.8) + vec3<f32>(1.0) * spec * 0.5;
    
    return vec4<f32>(color, 1.0);
}}
"#
    )
}
