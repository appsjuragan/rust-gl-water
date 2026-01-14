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
