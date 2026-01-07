//! Water simulation shaders - GPGPU compute/fragment shaders for wave simulation

/// Drop addition shader - adds ripples at a point
pub const DROP_SHADER: &str = r#"
// Vertex shader for fullscreen quad
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );
    
    var out: VertexOutput;
    let pos = positions[vertex_index];
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + 0.5;
    return out;
}

struct DropUniforms {
    center: vec2<f32>,
    radius: f32,
    strength: f32,
    pool_size: vec2<f32>,
    _padding: vec2<f32>,
}

const PI: f32 = 3.141592653589793;

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: DropUniforms;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var info = textureSample(input_texture, texture_sampler, in.uv);
    
    // center is in normalized -1..1 space, convert to UV
    let center_uv = uniforms.center * 0.5 + 0.5;
    let uv_vector = center_uv - in.uv;
    let world_vector = uv_vector * uniforms.pool_size;
    
    let dist = length(world_vector);
    var drop = max(0.0, 1.0 - dist / uniforms.radius);
    drop = 0.5 - cos(drop * PI) * 0.5;
    
    info.r += drop * uniforms.strength;
    
    return info;
}
"#;

/// Wave update shader - propagates waves using wave equation
pub const UPDATE_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );
    
    var out: VertexOutput;
    let pos = positions[vertex_index];
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + 0.5;
    return out;
}

struct UpdateUniforms {
    delta: vec2<f32>,
    pool_size: vec2<f32>,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: UpdateUniforms;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var info = textureSample(input_texture, texture_sampler, in.uv);
    
    let dx = vec2<f32>(uniforms.delta.x, 0.0);
    let dy = vec2<f32>(0.0, uniforms.delta.y);
    
    let u = info.r;
    let u_right = textureSample(input_texture, texture_sampler, in.uv + dx).r;
    let u_left = textureSample(input_texture, texture_sampler, in.uv - dx).r;
    let u_up = textureSample(input_texture, texture_sampler, in.uv + dy).r;
    let u_down = textureSample(input_texture, texture_sampler, in.uv - dy).r;
    
    // Weighted Laplacian for non-square aspect ratio
    let fx = 1.0 / (uniforms.pool_size.x * uniforms.pool_size.x);
    let fy = 1.0 / (uniforms.pool_size.y * uniforms.pool_size.y);
    
    let spatial_average = ((u_left + u_right) * fx + (u_up + u_down) * fy) / (2.0 * (fx + fy));
    
    // Wave equation update
    info.g += (spatial_average - u) * 2.0;
    info.g *= 0.995; // Damping
    info.r += info.g;
    
    return info;
}
"#;

/// Normal calculation shader
pub const NORMAL_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );
    
    var out: VertexOutput;
    let pos = positions[vertex_index];
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + 0.5;
    return out;
}

struct NormalUniforms {
    delta: vec2<f32>,
    pool_size: vec2<f32>,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: NormalUniforms;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var info = textureSample(input_texture, texture_sampler, in.uv);
    
    // Physical dx and dy
    let dx_phys = uniforms.pool_size.x * uniforms.delta.x;
    let dy_phys = uniforms.pool_size.y * uniforms.delta.y;
    
    let height_right = textureSample(input_texture, texture_sampler, vec2<f32>(in.uv.x + uniforms.delta.x, in.uv.y)).r;
    let height_up = textureSample(input_texture, texture_sampler, vec2<f32>(in.uv.x, in.uv.y + uniforms.delta.y)).r;
    
    let dx_vec = vec3<f32>(dx_phys, height_right - info.r, 0.0);
    let dy_vec = vec3<f32>(0.0, height_up - info.r, dy_phys);
    
    let normal = normalize(cross(dy_vec, dx_vec));
    info.b = normal.x;
    info.a = normal.z;
    
    return info;
}
"#;

/// Sphere volume displacement shader
pub const SPHERE_VOLUME_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );
    
    var out: VertexOutput;
    let pos = positions[vertex_index];
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + 0.5;
    return out;
}

struct SphereVolumeUniforms {
    old_center: vec4<f32>,
    new_center: vec4<f32>,
    radius: f32,
    strength: f32,
    pool_size: vec2<f32>,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: SphereVolumeUniforms;

fn volume_in_sphere(center: vec3<f32>, uv: vec2<f32>) -> f32 {
    // Convert UV to world position
    let pos = vec3<f32>(
        (uv.x * 2.0 - 1.0) * uniforms.pool_size.x / 2.0,
        0.0,
        (uv.y * 2.0 - 1.0) * uniforms.pool_size.y / 2.0
    );
    
    // Convert normalized center to world position
    let world_center = vec3<f32>(
        center.x * uniforms.pool_size.x / 2.0,
        center.y,
        center.z * uniforms.pool_size.y / 2.0
    );
    
    let to_center = pos - world_center;
    let t = length(to_center) / uniforms.radius;
    let dy = exp(-pow(t * 1.5, 6.0));
    let y_min = min(0.0, world_center.y - dy);
    let y_max = min(max(0.0, world_center.y + dy), y_min + 2.0 * dy);
    
    return (y_max - y_min) * uniforms.strength;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var info = textureSample(input_texture, texture_sampler, in.uv);
    
    info.r += volume_in_sphere(uniforms.old_center.xyz, in.uv);
    info.r -= volume_in_sphere(uniforms.new_center.xyz, in.uv);
    
    return info;
}
"#;
