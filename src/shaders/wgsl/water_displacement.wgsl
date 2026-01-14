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

fn volume_in_shape(center: vec3<f32>, uv: vec2<f32>, strength: f32) -> f32 {
    let norm_x = uv.x * 2.0 - 1.0;
    let norm_z = uv.y * 2.0 - 1.0;
    let dx = norm_x - center.x;
    let dz = norm_z - center.z;
    let water_level = 0.0;
    let norm_radius = uniforms.radius / (uniforms.pool_size.x / 2.0);
    
    if (uniforms.shape_type == 0) {
        let r = norm_radius;
        let d2 = dx*dx + dz*dz;
        if (d2 > r*r) { return 0.0; }
        let h_half = sqrt(max(0.0, uniforms.radius*uniforms.radius - d2 * (uniforms.pool_size.x / 2.0) * (uniforms.pool_size.x / 2.0)));
        let top = center.y + h_half;
        let bot = center.y - h_half;
        return max(0.0, min(top, water_level) - bot) * strength;
    }
    
    if (uniforms.shape_type == 1) {
        let R = 0.7 * norm_radius;
        let tube = 0.3 * norm_radius;
        let dist = sqrt(dx*dx + dz*dz);
        let dist_from_ring = abs(dist - R);
        if (dist_from_ring > tube) { return 0.0; }
        let h_half = sqrt(max(0.0, (0.3*uniforms.radius)*(0.3*uniforms.radius) - (dist_from_ring*(uniforms.pool_size.x/2.0))*(dist_from_ring*(uniforms.pool_size.x/2.0))));
        let top = center.y + h_half;
        let bot = center.y - h_half;
        return max(0.0, min(top, water_level) - bot) * strength;
    }
    
    if (uniforms.shape_type == 3) {
         let s = 0.577 * norm_radius;
         if (abs(dx) > s || abs(dz) > s) { return 0.0; }
         let world_s = 0.577 * uniforms.radius;
         let top = center.y + world_s;
         let bot = center.y - world_s;
         return max(0.0, min(top, water_level) - bot) * strength;
    }
    
    // Scan fallback for others
    let world_dx = dx * (uniforms.pool_size.x / 2.0);
    let world_dz = dz * (uniforms.pool_size.y / 2.0);
    let steps = 20;
    let step_size = (uniforms.radius * 2.0) / f32(steps);
    var thickness = 0.0;
    var current_y = -uniforms.radius; 
    for (var i = 0; i < steps; i++) {
        let p = vec3<f32>(world_dx, current_y, world_dz);
        let d = get_shape_dist(p, uniforms.shape_type, uniforms.radius);
        if (d < 0.0 && center.y + current_y < water_level) { thickness += step_size; }
        current_y += step_size;
    }
    return thickness * strength;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var info = textureSample(input_texture, texture_sampler, in.uv);
    for (var i = 0; i < uniforms.object_count; i++) {
        let obj = uniforms.objects[i];
        info.r += volume_in_shape(obj.old_center.xyz, in.uv, obj.strength);
        info.r -= volume_in_shape(obj.new_center.xyz, in.uv, obj.strength);
    }
    return info;
}
