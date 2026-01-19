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
    rotation: vec4<f32>,
    strength: f32,
    _pad_a: f32,
    _pad_b: f32,
    _pad_c: f32,
}

struct SphereVolumeUniforms {
    objects: array<ObjectTransition, 64>,
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
@group(0) @binding(2) var<uniform> displacement_uniforms: SphereVolumeUniforms;

fn volume_in_shape(center: vec3<f32>, rotation: vec4<f32>, uv: vec2<f32>, strength: f32) -> f32 {
    let norm_x = uv.x * 2.0 - 1.0;
    let norm_z = uv.y * 2.0 - 1.0;
    let dx = norm_x - center.x;
    let dz = norm_z - center.z;
    let water_level = 0.0;
    
    // World space offsets from center
    let world_dx = dx * (displacement_uniforms.pool_size.x / 2.0);
    let world_dz = dz * (displacement_uniforms.pool_size.y / 2.0);
    
    // Bounding box check (optimization)
    if (abs(world_dx) > displacement_uniforms.radius || abs(world_dz) > displacement_uniforms.radius) {
        return 0.0;
    }

    let inv_rotation = vec4<f32>(-rotation.xyz, rotation.w);
    let steps = 20;
    let step_size = (displacement_uniforms.radius * 2.0) / f32(steps);
    var thickness = 0.0;
    var current_y = -displacement_uniforms.radius; 
    
    for (var i = 0; i < steps; i++) {
        let p_rel = vec3<f32>(world_dx, current_y, world_dz);
        let p_local = rotate_vector(p_rel, inv_rotation);
        
        let d = get_shape_dist(p_local, displacement_uniforms.shape_type, displacement_uniforms.radius);
        
        if (d < 0.0 && center.y + current_y < water_level) { 
            thickness += step_size; 
        }
        current_y += step_size;
    }
    return thickness * strength;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var info = textureSample(input_texture, texture_sampler, in.uv);
    
    for (var i = 0; i < displacement_uniforms.object_count; i++) {
        let obj = displacement_uniforms.objects[i];
        info.r += volume_in_shape(obj.old_center.xyz, obj.rotation, in.uv, obj.strength);
        info.r -= volume_in_shape(obj.new_center.xyz, obj.rotation, in.uv, obj.strength);
    }
    
    return info;
}
