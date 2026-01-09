//! Sphere shader - renders the floating sphere

use super::common::{COMMON_UNIFORMS, HELPER_FUNCTIONS};

pub fn sphere_shader() -> String {
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
    @location(1) world_normal: vec3<f32>,
    @location(2) view_pos: vec3<f32>,
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
fn vs_main(in: VertexInput, @builtin(instance_index) instance_idx: u32) -> VertexOutput {{
    var out: VertexOutput;
    
    var center: vec3<f32>;
    if (instance_idx == 0u) {{
        center = uniforms.sphere_centers[0].xyz;
    }} else if (instance_idx == 1u) {{
        center = uniforms.sphere_centers[1].xyz;
    }} else if (instance_idx == 2u) {{
        center = uniforms.sphere_centers[2].xyz;
    }} else if (instance_idx == 3u) {{
        center = uniforms.sphere_centers[3].xyz;
    }} else {{
        center = uniforms.sphere_centers[4].xyz;
    }}
    
    // Transform unit sphere to world position
    let world_pos = in.position * uniforms.sphere_radius + center;
    out.world_pos = world_pos;
    out.world_normal = in.normal;
    
    out.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.view_pos = camera.eye.xyz;
    
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
fn fs_main(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {{
    let light_dir = normalize(-uniforms.light_dir.xyz);
    let normal = normalize(in.world_normal);
    let view_dir = normalize(in.world_pos - in.view_pos); // Vector from eye to point
    
    // Get material properties based on texture type
    let mat = get_material_props(uniforms.texture_type);
    
    // Fresnel with material-dependent specularity
    let base_fresnel = mat.specularity * 0.2;
    let fresnel = base_fresnel + (1.0 - base_fresnel) * pow(1.0 - max(0.0, dot(-view_dir, normal)), 2.0 + mat.roughness * 3.0);
    
    // Reflection
    let reflect_dir = reflect(view_dir, normal);
    var reflect_color = get_surface_ray_color(in.world_pos, reflect_dir, ABOVE_WATER_COLOR);
    
    // Tint reflection with material color for metallic materials
    if (uniforms.texture_type == 2) {{ // Steel
        reflect_color *= mat.base_color;
    }}
    
    // Refraction (Air -> Material)
    let ior_ratio_in = IOR_AIR / mat.ior;
    let refract_dir_in = refract(view_dir, normal, ior_ratio_in);
    
    var refract_color = mat.base_color * 0.3; // Base color for TIR or opaque materials
    
    let radius = uniforms.sphere_radius;
    let shape_type = uniforms.shape_type;
    
    // For highly absorbing materials (wood), skip complex refraction
    let is_transparent = mat.absorption.x < 0.5;
    
    if (length(refract_dir_in) > 0.001 && is_transparent) {{
        // Need to find which sphere we are in for local coordinates
        var center: vec3<f32>;
        var best_dist = 1e30;
        for (var i = 0; i < uniforms.object_count; i++) {{
            let c = uniforms.sphere_centers[i].xyz;
            let d = length(in.world_pos - c);
            if (d < best_dist) {{
                best_dist = d;
                center = c;
            }}
        }}
        
        let local_origin = in.world_pos - center;
        let t_exit = get_exit_dist_shape(local_origin, refract_dir_in, shape_type, radius);
        
        if (t_exit > 0.001) {{
            let exit_point = in.world_pos + refract_dir_in * t_exit;
            
            let e = 0.001;
            let local_exit = exit_point - center;
            let dx = get_shape_dist(local_exit + vec3<f32>(e,0.0,0.0), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(e,0.0,0.0), shape_type, radius);
            let dy = get_shape_dist(local_exit + vec3<f32>(0.0,e,0.0), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(0.0,e,0.0), shape_type, radius);
            let dz = get_shape_dist(local_exit + vec3<f32>(0.0,0.0,e), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(0.0,0.0,e), shape_type, radius);
            let exit_normal = normalize(vec3<f32>(dx, dy, dz)); 
            
            let water_level = textureSample(water_texture, water_sampler, exit_point.xz / (uniforms.pool_size * 2.0) + 0.5).r;
            
            var ior_ratio_out = mat.ior / IOR_AIR; // Material -> Air
            if (exit_point.y < water_level) {{
                ior_ratio_out = mat.ior / IOR_WATER; // Material -> Water
            }}
            
            let refract_dir_out = refract(refract_dir_in, -exit_normal, ior_ratio_out); 
            
            if (length(refract_dir_out) > 0.0) {{
                refract_color = get_surface_ray_color(exit_point, refract_dir_out, ABOVE_WATER_COLOR);
                
                // Attenuate color based on distance and material absorption
                refract_color *= exp(-mat.absorption * t_exit * 8.0);
                // Tint with base color
                refract_color *= mat.base_color;
            }} else {{
                // Total Internal Reflection
                refract_color = reflect_color * mat.base_color;
            }}
        }}
    }} else if (!is_transparent) {{
        // For opaque materials (wood), use a simple diffuse model
        let diffuse = max(0.0, dot(normal, light_dir));
        let ambient = 0.3;
        refract_color = mat.base_color * (ambient + diffuse * 0.7) * uniforms.light_color.rgb;
    }}
    
    // Specular highlight with material-dependent intensity
    let spec_power = 20.0 + (1.0 - mat.roughness) * 200.0;
    let spec = pow(max(dot(reflect_dir, light_dir), 0.0), spec_power) * mat.specularity;
    
    let final_color = mix(refract_color, reflect_color, fresnel) + uniforms.light_color.rgb * spec;
    
    return vec4<f32>(final_color, 1.0);
}}
"#
    )
}
