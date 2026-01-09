//! Pool (cube walls) shader

use super::common::{COMMON_UNIFORMS, HELPER_FUNCTIONS};

pub fn pool_shader() -> String {
    format!(
        r#"
{COMMON_UNIFORMS}

{HELPER_FUNCTIONS}

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
    @location(1) uv: vec2<f32>,
}}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> uniforms: CommonUniforms;

@group(1) @binding(0) var water_texture: texture_2d<f32>;
@group(1) @binding(1) var water_sampler: sampler;
@group(1) @binding(2) var tile_texture: texture_2d<f32>;
@group(1) @binding(3) var tile_sampler: sampler;
@group(1) @binding(4) var caustic_texture: texture_2d<f32>;
@group(1) @binding(5) var caustic_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {{
    var out: VertexOutput;
    
    // Scale cube to pool dimensions
    var pos = in.position;
    pos.x *= uniforms.pool_size.x;
    pos.z *= uniforms.pool_size.y;
    
    let height = uniforms.pool_height + uniforms.wall_height;
    pos.y *= max(height / 2.0, 0.01);
    pos.y += (uniforms.wall_height - uniforms.pool_height) / 2.0;
    
    out.world_pos = pos;
    out.position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.uv = in.uv;
    
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let position = in.world_pos;
    let wall_height = uniforms.wall_height;
    let pool_size = uniforms.pool_size;
    
    // Discard above-water wall tops
    if position.y > wall_height - 0.001 {{
        discard;
    }}
    
    let coord = position.xz / (pool_size * 2.0) + 0.5;
    let water_info = textureSample(water_texture, water_sampler, coord);
    let caustic = textureSample(caustic_texture, caustic_sampler, coord);
    
    // Calculate tile coordinates based on which face we're on
    var tile_coord: vec2<f32>;
    if abs(position.x) > pool_size.x - 0.01 {{
        tile_coord = position.yz * 0.5 + vec2<f32>(1.0, 0.5);
    }} else if abs(position.z) > pool_size.y - 0.01 {{
        tile_coord = position.yx * 0.5 + vec2<f32>(1.0, 0.5);
    }} else {{
        tile_coord = position.xz * 0.5 + 0.5;
    }}
    
    let tile_color = textureSample(tile_texture, tile_sampler, tile_coord).rgb;
    var color = get_wall_color(position, uniforms, water_info, caustic, tile_color);
    
    // Underwater tint
    if position.y < water_info.r {{
        color *= UNDERWATER_COLOR * 1.2;
    }}
    
    return vec4<f32>(color, 1.0);
}}
"#
    )
}
