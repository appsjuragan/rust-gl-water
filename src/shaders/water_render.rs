//! Water surface rendering shader

use super::common::{COMMON_UNIFORMS, HELPER_FUNCTIONS};

pub fn water_shader() -> String {
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
    @location(1) uv: vec2<f32>,
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
@group(1) @binding(6) var sky_texture: texture_2d<f32>;
@group(1) @binding(7) var sky_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {{
    var out: VertexOutput;
    
    let info = textureSampleLevel(water_texture, water_sampler, in.uv, 0.0);
    
    var pos = vec3<f32>(
        in.position.x,
        info.r,
        in.position.z
    );
    
    // Scale to pool size
    pos.x *= uniforms.pool_size.x;
    pos.z *= uniforms.pool_size.y;
    
    out.world_pos = pos;
    out.position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.uv = in.uv;
    
    return out;
}}

fn get_surface_ray_color(origin: vec3<f32>, ray: vec3<f32>, water_color: vec3<f32>) -> vec3<f32> {{
    var color: vec3<f32>;
    let pool_size = uniforms.pool_size;
    let pool_height = uniforms.pool_height;
    let wall_height = uniforms.wall_height;
    let light = uniforms.light_dir.xyz;
    
    let cube_min = vec3<f32>(-pool_size.x, -pool_height, -pool_size.y);
    let cube_max = vec3<f32>(pool_size.x, wall_height, pool_size.y);
    let t = intersect_cube(origin, ray, cube_min, cube_max);
    let hit = origin + ray * t.y;
    
    if ray.y < 0.0 {{
        // Looking down - hit floor/walls
        let coord = hit.xz / (pool_size * 2.0) + 0.5;
        let water_info = textureSample(water_texture, water_sampler, coord);
        let caustic = textureSample(caustic_texture, caustic_sampler, coord);
        
        var tile_coord: vec2<f32>;
        if abs(hit.x) > pool_size.x - 0.01 {{
            tile_coord = hit.yz * 0.5 + vec2<f32>(1.0, 0.5);
        }} else if abs(hit.z) > pool_size.y - 0.01 {{
            tile_coord = hit.yx * 0.5 + vec2<f32>(1.0, 0.5);
        }} else {{
            tile_coord = hit.xz * 0.5 + 0.5;
        }}
        let tile_color = textureSample(tile_texture, tile_sampler, tile_coord).rgb;
        color = get_wall_color(hit, uniforms, water_info, caustic, tile_color);
    }} else {{
        // Looking up
        if hit.y < wall_height - 0.001 {{
            // Hit wall above water
            let coord = hit.xz / (pool_size * 2.0) + 0.5;
            let water_info = textureSample(water_texture, water_sampler, coord);
            let caustic = textureSample(caustic_texture, caustic_sampler, coord);
            
            var tile_coord: vec2<f32>;
            if abs(hit.x) > pool_size.x - 0.01 {{
                tile_coord = hit.yz * 0.5 + vec2<f32>(1.0, 0.5);
            }} else {{
                tile_coord = hit.yx * 0.5 + vec2<f32>(1.0, 0.5);
            }}
            let tile_color = textureSample(tile_texture, tile_sampler, tile_coord).rgb;
            color = get_wall_color(hit, uniforms, water_info, caustic, tile_color);
        }} else {{
            // Hit sky
            let sky_uv = ray.xz * 0.5 + 0.5;
            color = textureSample(sky_texture, sky_sampler, sky_uv).rgb;
            // Sun highlight
            color += vec3<f32>(pow(max(0.0, dot(light, ray)), 5000.0)) * vec3<f32>(10.0, 8.0, 6.0);
        }}
    }}
    
    if ray.y < 0.0 {{
        color *= water_color;
    }}
    
    return color;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let position = in.world_pos;
    let pool_size = uniforms.pool_size;
    
    var coord = position.xz / (pool_size * 2.0) + 0.5;
    var info = textureSample(water_texture, water_sampler, coord);
    
    // Refine coordinates based on normal
    for (var i = 0; i < 5; i++) {{
        coord += info.ba * 0.005;
        info = textureSample(water_texture, water_sampler, coord);
    }}
    
    let normal = vec3<f32>(info.b, sqrt(1.0 - dot(info.ba, info.ba)), info.a);
    let incoming_ray = normalize(position - camera.eye.xyz);
    
    var reflected_ray: vec3<f32>;
    var refracted_ray: vec3<f32>;
    var fresnel: f32;
    
    if dot(incoming_ray, normal) < 0.0 {{
        // Above water
        reflected_ray = reflect(incoming_ray, normal);
        refracted_ray = refract(incoming_ray, normal, IOR_AIR / IOR_WATER);
        fresnel = mix(0.25, 1.0, pow(1.0 - dot(normal, -incoming_ray), 3.0));
    }} else {{
        // Below water
        let flipped_normal = -normal;
        reflected_ray = reflect(incoming_ray, flipped_normal);
        refracted_ray = refract(incoming_ray, flipped_normal, IOR_WATER / IOR_AIR);
        fresnel = mix(0.25, 1.0, pow(1.0 - dot(flipped_normal, -incoming_ray), 3.0));
    }}
    
    let reflected_color = get_surface_ray_color(position, reflected_ray, ABOVE_WATER_COLOR);
    var refracted_color = vec3<f32>(0.0);
    
    if length(refracted_ray) > 0.001 {{
        refracted_color = get_surface_ray_color(position, refracted_ray, ABOVE_WATER_COLOR);
    }} else {{
        fresnel = 1.0; // Total internal reflection
    }}
    
    let final_color = mix(refracted_color, reflected_color, fresnel);
    
    return vec4<f32>(final_color, 1.0);
}}
"#
    )
}
