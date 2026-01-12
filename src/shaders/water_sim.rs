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
    
    // 9-point Laplacian for better isotropic propagation
    let u_r = textureSample(input_texture, texture_sampler, in.uv + dx).r;
    let u_l = textureSample(input_texture, texture_sampler, in.uv - dx).r;
    let u_u = textureSample(input_texture, texture_sampler, in.uv + dy).r;
    let u_d = textureSample(input_texture, texture_sampler, in.uv - dy).r;
    
    let u_ur = textureSample(input_texture, texture_sampler, in.uv + dx + dy).r;
    let u_ul = textureSample(input_texture, texture_sampler, in.uv - dx + dy).r;
    let u_dr = textureSample(input_texture, texture_sampler, in.uv + dx - dy).r;
    let u_dl = textureSample(input_texture, texture_sampler, in.uv - dx - dy).r;
    
    // Weights: 0.2 for direct neighbors, 0.05 for diagonals
    let laplacian = (u_r + u_l + u_u + u_d) * 0.2 + (u_ur + u_ul + u_dr + u_dl) * 0.05 - u;
    
    // Wave equation update with slight numerical damping
    // info.g is velocity, info.r is height
    info.g += laplacian * 1.8; // Stiffness
    info.g *= 0.992;           // Velocity damping
    info.r += info.g;
    info.r *= 0.998;           // Height damping (helps stability)
    
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

struct ObjectTransition {
    old_center: vec4<f32>,
    new_center: vec4<f32>,
    strength: f32,
    _pad_a: f32,
    _pad_b: f32,
    _pad_c: f32,
}

struct SphereVolumeUniforms {
    objects: array<ObjectTransition, 5>,
    radius: f32,
    _pad0: f32,
    pool_size: vec2<f32>,
    shape_type: i32,
    object_count: i32,
    _pad1: f32,
    _pad2: f32,
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


fn volume_in_shape(center: vec3<f32>, uv: vec2<f32>, strength: f32) -> f32 {
    // Convert UV (0..1) to normalized space (-1..1) to match center coordinates
    let norm_x = uv.x * 2.0 - 1.0;
    let norm_z = uv.y * 2.0 - 1.0;
    
    // center.x and center.z are already in normalized -1..1 space
    let dx = norm_x - center.x;
    let dz = norm_z - center.z;
    let water_level = 0.0;
    
    // Scale radius to normalized space
    let norm_radius = uniforms.radius / (uniforms.pool_size.x / 2.0);
    
    // Analytic Sphere
    if (uniforms.shape_type == 0) {
        let r = norm_radius;  // Use normalized radius for XZ check
        let d2 = dx*dx + dz*dz;
        if (d2 > r*r) { return 0.0; }
        
        // For height calculation, use world-space radius
        let world_r = uniforms.radius;
        let h_half = sqrt(world_r*world_r - d2 * (uniforms.pool_size.x / 2.0) * (uniforms.pool_size.x / 2.0));
        let top = center.y + h_half;
        let bot = center.y - h_half;
        
        let actual_top = min(top, water_level);
        let submerged_h = max(0.0, actual_top - bot);
        
        return submerged_h * strength;
    }
    
    // Analytic Torus
    if (uniforms.shape_type == 1) {
        let R = 0.7 * norm_radius;
        let tube = 0.3 * norm_radius;
        let dist = sqrt(dx*dx + dz*dz);
        let dist_from_ring = abs(dist - R);
        if (dist_from_ring > tube) { return 0.0; }
        
        let world_tube = 0.3 * uniforms.radius;
        let world_dist_from_ring = dist_from_ring * (uniforms.pool_size.x / 2.0);
        let h_half = sqrt(world_tube*world_tube - world_dist_from_ring*world_dist_from_ring);
        let top = center.y + h_half;
        let bot = center.y - h_half;
        
        let actual_top = min(top, water_level);
        let submerged_h = max(0.0, actual_top - bot);
        
        return submerged_h * strength;
    }
    
    // Analytic Cube
    if (uniforms.shape_type == 3) {
         let s = 0.577 * norm_radius;
         let edge = 0.02;
         let mask_x = 1.0 - smoothstep(s - edge, s, abs(dx));
         let mask_z = 1.0 - smoothstep(s - edge, s, abs(dz));
         
         let world_s = 0.577 * uniforms.radius;
         let top = center.y + world_s;
         let bot = center.y - world_s;
         
         let actual_top = min(top, water_level);
         let submerged_h = max(0.0, actual_top - bot);
         
         return submerged_h * mask_x * mask_z * strength;
    }
    
    // Fallback Scan (Tetrahedron)
    if (dx*dx + dz*dz > norm_radius * norm_radius * 2.5) {
        return 0.0;
    }
    
    // Scale dx, dz back to world space for SDF evaluation
    let world_dx = dx * (uniforms.pool_size.x / 2.0);
    let world_dz = dz * (uniforms.pool_size.y / 2.0);
    
    let steps = 20;
    let step_size = (uniforms.radius * 2.0) / f32(steps);
    var thickness = 0.0;
    var current_y = -uniforms.radius; 
    let smoothing = step_size * 0.8;
    
    for (var i = 0; i < steps; i++) {
        let p = vec3<f32>(world_dx, current_y, world_dz);
        let d = get_shape_dist(p, uniforms.shape_type, uniforms.radius);
        
        let weight = smoothstep(smoothing, -smoothing, d);
        
        let world_y = center.y + current_y;
        if (world_y < water_level) {
            thickness += weight * step_size;
        }
        
        current_y += step_size;
    }
    
    return thickness * strength * 4.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var info = textureSample(input_texture, texture_sampler, in.uv);
    
    if (uniforms.object_count > 0) {
        let obj = uniforms.objects[0];
        info.r += volume_in_shape(obj.old_center.xyz, in.uv, obj.strength);
        info.r -= volume_in_shape(obj.new_center.xyz, in.uv, obj.strength);
    }
    if (uniforms.object_count > 1) {
        let obj = uniforms.objects[1];
        info.r += volume_in_shape(obj.old_center.xyz, in.uv, obj.strength);
        info.r -= volume_in_shape(obj.new_center.xyz, in.uv, obj.strength);
    }
    if (uniforms.object_count > 2) {
        let obj = uniforms.objects[2];
        info.r += volume_in_shape(obj.old_center.xyz, in.uv, obj.strength);
        info.r -= volume_in_shape(obj.new_center.xyz, in.uv, obj.strength);
    }
    if (uniforms.object_count > 3) {
        let obj = uniforms.objects[3];
        info.r += volume_in_shape(obj.old_center.xyz, in.uv, obj.strength);
        info.r -= volume_in_shape(obj.new_center.xyz, in.uv, obj.strength);
    }
    if (uniforms.object_count > 4) {
        let obj = uniforms.objects[4];
        info.r += volume_in_shape(obj.old_center.xyz, in.uv, obj.strength);
        info.r -= volume_in_shape(obj.new_center.xyz, in.uv, obj.strength);
    }
    
    return info;
}
"#;
