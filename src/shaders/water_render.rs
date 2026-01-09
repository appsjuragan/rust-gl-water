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
    
    var t_final = t.y;
    var is_shape = false;
    
    // Check shape intersection
    let t_shape = intersect_shape_any(origin, ray, uniforms.sphere_center.xyz, uniforms.sphere_radius, uniforms.shape_type);
    if (t_shape > 0.0 && t_shape < t_final) {{
        t_final = t_shape;
        is_shape = true;
    }}
    
    let hit = origin + ray * t_final;
    
    if (is_shape) {{
        // Material-dependent refraction through the object
        let shape_type = uniforms.shape_type;
        let radius = uniforms.sphere_radius;
        let center = uniforms.sphere_center.xyz;
        let p = hit - center;
        
        // Get material properties
        let mat = get_material_props(uniforms.texture_type);
        
        // Calculate entry normal using SDF gradient
        let e = 0.001;
        let dx = get_shape_dist(p + vec3<f32>(e,0.0,0.0), shape_type, radius) - get_shape_dist(p - vec3<f32>(e,0.0,0.0), shape_type, radius);
        let dy = get_shape_dist(p + vec3<f32>(0.0,e,0.0), shape_type, radius) - get_shape_dist(p - vec3<f32>(0.0,e,0.0), shape_type, radius);
        let dz = get_shape_dist(p + vec3<f32>(0.0,0.0,e), shape_type, radius) - get_shape_dist(p - vec3<f32>(0.0,0.0,e), shape_type, radius);
        let normal = normalize(vec3<f32>(dx, dy, dz));
        
        // Fresnel effect with material-dependent specularity
        let base_fresnel = mat.specularity * 0.15;
        let fresnel = base_fresnel + (1.0 - base_fresnel) * pow(1.0 - max(0.0, dot(-ray, normal)), 2.5 + mat.roughness * 2.0);
        
        // Reflection off surface
        let reflect_dir = reflect(ray, normal);
        
        // Refraction into material (water IOR -> material IOR)
        let ior_water_to_mat = IOR_WATER / mat.ior;
        let refract_dir_in = refract(ray, normal, ior_water_to_mat);
        
        var refract_color = mat.base_color * 0.3; // Default for opaque/TIR
        var reflect_color = vec3<f32>(0.0);
        
        // Check if material is transparent enough for refraction
        let is_transparent = mat.absorption.x < 0.5;
        
        if (length(refract_dir_in) > 0.001 && is_transparent) {{
            // Find exit point through material
            let local_origin = hit - center;
            let t_exit = get_exit_dist_shape(local_origin, refract_dir_in, shape_type, radius);
            
            if (t_exit > 0.001) {{
                let exit_point = hit + refract_dir_in * t_exit;
                
                // Calculate exit normal
                let local_exit = exit_point - center;
                let dx2 = get_shape_dist(local_exit + vec3<f32>(e,0.0,0.0), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(e,0.0,0.0), shape_type, radius);
                let dy2 = get_shape_dist(local_exit + vec3<f32>(0.0,e,0.0), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(0.0,e,0.0), shape_type, radius);
                let dz2 = get_shape_dist(local_exit + vec3<f32>(0.0,0.0,e), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(0.0,0.0,e), shape_type, radius);
                let exit_normal = normalize(vec3<f32>(dx2, dy2, dz2));
                
                // Refract out of material back into water
                let ior_mat_to_water = mat.ior / IOR_WATER;
                let refract_dir_out = refract(refract_dir_in, -exit_normal, ior_mat_to_water);
                
                if (length(refract_dir_out) > 0.001) {{
                    // Trace ray to background (pool floor/walls)
                    let t_bg = intersect_cube(exit_point, refract_dir_out, cube_min, cube_max);
                    let bg_hit = exit_point + refract_dir_out * t_bg.y;
                    
                    let coord = bg_hit.xz / (pool_size * 2.0) + 0.5;
                    let water_info = textureSample(water_texture, water_sampler, coord);
                    let caustic = textureSample(caustic_texture, caustic_sampler, coord);
                    
                    var tile_coord: vec2<f32>;
                    if abs(bg_hit.x) > pool_size.x - 0.01 {{
                        tile_coord = bg_hit.yz * 0.5 + vec2<f32>(1.0, 0.5);
                    }} else if abs(bg_hit.z) > pool_size.y - 0.01 {{
                        tile_coord = bg_hit.yx * 0.5 + vec2<f32>(1.0, 0.5);
                    }} else {{
                        tile_coord = bg_hit.xz * 0.5 + 0.5;
                    }}
                    let tile_color = textureSample(tile_texture, tile_sampler, tile_coord).rgb;
                    refract_color = get_wall_color(bg_hit, uniforms, water_info, caustic, tile_color);
                    
                    // Attenuate based on distance and material absorption
                    refract_color *= exp(-mat.absorption * t_exit * 6.0);
                    refract_color *= mat.base_color;
                }} else {{
                    // Total internal reflection
                    refract_color = mat.base_color * 0.4;
                }}
            }}
        }} else if (!is_transparent) {{
            // For opaque materials (wood), use diffuse lighting
            let diffuse = max(0.0, dot(normal, light));
            let ambient = 0.35;
            refract_color = mat.base_color * (ambient + diffuse * 0.65) * uniforms.light_color.rgb;
        }}
        
        // Calculate reflection color (trace reflected ray)
        let t_refl = intersect_cube(hit, reflect_dir, cube_min, cube_max);
        let refl_hit = hit + reflect_dir * t_refl.y;
        let refl_coord = refl_hit.xz / (pool_size * 2.0) + 0.5;
        let refl_water_info = textureSample(water_texture, water_sampler, refl_coord);
        let refl_caustic = textureSample(caustic_texture, caustic_sampler, refl_coord);
        var refl_tile_coord: vec2<f32>;
        if abs(refl_hit.x) > pool_size.x - 0.01 {{
            refl_tile_coord = refl_hit.yz * 0.5 + vec2<f32>(1.0, 0.5);
        }} else if abs(refl_hit.z) > pool_size.y - 0.01 {{
            refl_tile_coord = refl_hit.yx * 0.5 + vec2<f32>(1.0, 0.5);
        }} else {{
            refl_tile_coord = refl_hit.xz * 0.5 + 0.5;
        }}
        let refl_tile_color = textureSample(tile_texture, tile_sampler, refl_tile_coord).rgb;
        reflect_color = get_wall_color(refl_hit, uniforms, refl_water_info, refl_caustic, refl_tile_color);
        
        // Tint reflection for metallic materials
        if (uniforms.texture_type == 2) {{ // Steel
            reflect_color *= mat.base_color;
        }}
        
        // Specular highlight with material-dependent intensity
        let spec_power = 30.0 + (1.0 - mat.roughness) * 150.0;
        let spec = pow(max(0.0, dot(reflect_dir, light)), spec_power) * mat.specularity;
        
        // Mix refraction and reflection based on Fresnel
        color = mix(refract_color, reflect_color, fresnel) + uniforms.light_color.rgb * spec * 0.25;
        
    }} else if ray.y < 0.0 {{
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
            color += vec3<f32>(pow(max(0.0, dot(light, ray)), 1000.0)) * vec3<f32>(8.0, 6.0, 4.0);
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
