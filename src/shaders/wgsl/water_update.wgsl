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
    
    // Wave equation update - balanced for stability and speed
    // info.g is velocity, info.r is height
    info.g += laplacian * 2.0; // Stiffness (stable increase)
    info.g *= 0.99;            // Velocity damping
    info.r += info.g;
    info.r *= 0.99;            // Height damping (fast but stable equilibrium)
    
    return info;
}
