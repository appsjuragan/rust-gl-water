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
