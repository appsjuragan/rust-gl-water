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
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
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
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
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
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
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
    let height_left = textureSample(input_texture, texture_sampler, vec2<f32>(in.uv.x - uniforms.delta.x, in.uv.y)).r;
    let height_down = textureSample(input_texture, texture_sampler, vec2<f32>(in.uv.x, in.uv.y + uniforms.delta.y)).r;
    let height_up = textureSample(input_texture, texture_sampler, vec2<f32>(in.uv.x, in.uv.y - uniforms.delta.y)).r;
    
    let dx_vec = vec3<f32>(dx_phys * 2.0, height_right - height_left, 0.0);
    let dy_vec = vec3<f32>(0.0, height_down - height_up, dy_phys * 2.0);
    
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
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
    return out;
}

struct SphereVolumeUniforms {
    old_center: vec4<f32>,
    new_center: vec4<f32>,
    radius: f32,
    strength: f32,
    pool_size: vec2<f32>,
    shape_type: i32, // 0=Sphere, 1=Torus, 2=Tetrahedron, 3=Cube
    _padding: f32,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: SphereVolumeUniforms;

// --- SDF Functions (Copied from common.rs/helper) ---
fn sdSphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sdTorus(p: vec3<f32>, param: vec2<f32>) -> f32 {
    let t = vec2<f32>(param.x, param.y);
    let q = vec2<f32>(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

fn get_shape_dist(p: vec3<f32>, shape_type: i32, radius: f32) -> f32 {
    if (shape_type == 0) { // Sphere
        return sdSphere(p, radius);
    } else if (shape_type == 1) { // Torus
        return sdTorus(p, vec2<f32>(0.7 * radius, 0.3 * radius));
    } else if (shape_type == 2) { // Tetrahedron
        return (max(abs(p.x+p.y)-p.z, abs(p.x-p.y)+p.z) - 1.0 * radius) / sqrt(3.0); 
    } else { // Cube
        return sdBox(p, vec3<f32>(0.577 * radius));
    }
}


fn volume_in_shape(center: vec3<f32>, uv: vec2<f32>) -> f32 {
    // World position of this water column
    let world_x = (uv.x * 2.0 - 1.0) * uniforms.pool_size.x / 2.0;
    let world_z = (uv.y * 2.0 - 1.0) * uniforms.pool_size.y / 2.0;
    
    let dx = world_x - center.x;
    let dz = world_z - center.z;
    let water_level = 0.0;
    
    // Analytic Sphere
    if (uniforms.shape_type == 0) {
        let r = uniforms.radius;
        let d2 = dx*dx + dz*dz;
        if (d2 > r*r) { return 0.0; }
        
        let h_half = sqrt(r*r - d2);
        let top = center.y + h_half;
        let bot = center.y - h_half;
        
        let actual_top = min(top, water_level);
        let submerged_h = max(0.0, actual_top - bot);
        
        return submerged_h * uniforms.strength;
    }
    
    // Analytic Torus
    if (uniforms.shape_type == 1) {
        let R = 0.7 * uniforms.radius;
        let tube = 0.3 * uniforms.radius;
        let dist = sqrt(dx*dx + dz*dz);
        let dist_from_ring = abs(dist - R);
        if (dist_from_ring > tube) { return 0.0; }
        
        let h_half = sqrt(tube*tube - dist_from_ring*dist_from_ring);
        let top = center.y + h_half;
        let bot = center.y - h_half;
        
        let actual_top = min(top, water_level);
        let submerged_h = max(0.0, actual_top - bot);
        
        return submerged_h * uniforms.strength;
    }
    
    // Analytic Cube
    if (uniforms.shape_type == 3) {
         let s = 0.577 * uniforms.radius;
         // Soften cube edges slightly to prevent aliasing
         let edge = 0.02;
         let mask_x = 1.0 - smoothstep(s - edge, s, abs(dx));
         let mask_z = 1.0 - smoothstep(s - edge, s, abs(dz));
         
         let h_half = s; // Cube is symmetric vertically
         let top = center.y + h_half;
         let bot = center.y - h_half;
         
         let actual_top = min(top, water_level);
         let submerged_h = max(0.0, actual_top - bot);
         
         return submerged_h * mask_x * mask_z * uniforms.strength;
    }
    
    // Fallback Scan (Tetrahedron)
    if (dx*dx + dz*dz > uniforms.radius * uniforms.radius * 2.5) {
        return 0.0;
    }
    
    let steps = 20;
    let step_size = (uniforms.radius * 2.0) / f32(steps);
    var thickness = 0.0;
    var current_y = -uniforms.radius; 
    let smoothing = step_size * 0.8;
    
    for (var i = 0; i < steps; i++) {
        let p = vec3<f32>(dx, current_y, dz);
        let d = get_shape_dist(p, uniforms.shape_type, uniforms.radius);
        
        // Soft accumulation
        let weight = smoothstep(smoothing, -smoothing, d);
        
        // Check if this sample is underwater
        let world_y = center.y + current_y;
        if (world_y < water_level) {
            thickness += weight * step_size;
        }
        
        current_y += step_size;
    }
    
    return thickness * uniforms.strength * 4.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var info = textureSample(input_texture, texture_sampler, in.uv);
    
    info.r += volume_in_shape(uniforms.old_center.xyz, in.uv);
    info.r -= volume_in_shape(uniforms.new_center.xyz, in.uv);
    
    return info;
}
"#;
