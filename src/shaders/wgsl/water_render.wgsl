struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

@group(1) @binding(0) var water_texture: texture_2d<f32>;
@group(1) @binding(1) var water_sampler: sampler;
@group(1) @binding(2) var tile_texture: texture_2d<f32>;
@group(1) @binding(3) var tile_sampler: sampler;
@group(1) @binding(4) var caustic_texture: texture_2d<f32>;
@group(1) @binding(5) var caustic_sampler: sampler;
@group(1) @binding(6) var sky_texture: texture_2d<f32>;
@group(1) @binding(7) var sky_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let info = textureSampleLevel(water_texture, water_sampler, in.uv, 0.0);
    var pos = vec3<f32>(in.position.x, info.r, in.position.z);
    pos.x *= uniforms.pool_size.x;
    pos.z *= uniforms.pool_size.y;
    out.world_pos = pos;
    out.position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.uv = in.uv;
    return out;
}

fn get_surface_ray_color(origin: vec3<f32>, ray: vec3<f32>, water_color: vec3<f32>) -> vec3<f32> {
    var color: vec3<f32>;
    let pool_size = uniforms.pool_size;
    let pool_height = uniforms.pool_height;
    let wall_height = uniforms.wall_height;
    let light = uniforms.light_dir.xyz;
    
    var t_final = 1e30;
    
    if (uniforms.pool_shape == 2) { // Cylinder
        let res = intersect_cylinder_walls(origin, ray, pool_size.x, -pool_height, wall_height);
        if (res.x > 0.0) { t_final = res.x; }
        else if (res.y > 0.0) { t_final = res.y; }
    } else if (uniforms.pool_shape == 1) { // Frustum
        let s_min = pool_size * 0.7;
        let s_max = pool_size;
        let t = intersect_frustum(origin, ray, -pool_height, wall_height, s_min, s_max);
        t_final = t.y;
    } else { // Cube
        let cube_min = vec3<f32>(-pool_size.x, -pool_height, -pool_size.y);
        let cube_max = vec3<f32>(pool_size.x, wall_height, pool_size.y);
        let t = intersect_cube(origin, ray, cube_min, cube_max);
        t_final = t.y;
    }
    
    var is_shape = false;
    let shape_res = intersect_shape_any(origin, ray, uniforms);
    let t_shape = shape_res.x;
    var shape_idx = -1;
    if (t_shape > 0.0 && t_shape < t_final) {
        t_final = t_shape;
        is_shape = true;
        shape_idx = i32(shape_res.y);
    }
    
    let hit = origin + ray * t_final;
    
    if (is_shape) {
        let shape_type = uniforms.shape_type;
        let radius = uniforms.sphere_radius;
        let center = sphere_centers[shape_idx].xyz;
        let rotation = sphere_rotations[shape_idx];
        
        let inv_rotation = vec4<f32>(-rotation.xyz, rotation.w);
        let local_p = rotate_vector(hit - center, inv_rotation);
        let mat = get_material_props(uniforms.texture_type);
        let e = 0.001;
        let dx = get_shape_dist(local_p + vec3<f32>(e,0.0,0.0), shape_type, radius) - get_shape_dist(local_p - vec3<f32>(e,0.0,0.0), shape_type, radius);
        let dy = get_shape_dist(local_p + vec3<f32>(0.0,e,0.0), shape_type, radius) - get_shape_dist(local_p - vec3<f32>(0.0,e,0.0), shape_type, radius);
        let dz = get_shape_dist(local_p + vec3<f32>(0.0,0.0,e), shape_type, radius) - get_shape_dist(local_p - vec3<f32>(0.0,0.0,e), shape_type, radius);
        let local_normal = normalize(vec3<f32>(dx, dy, dz));
        let normal = rotate_vector(local_normal, rotation);
        
        let base_fresnel = mat.specularity * 0.15;
        let fresnel = base_fresnel + (1.0 - base_fresnel) * pow(1.0 - max(0.0, dot(-ray, normal)), 2.5 + mat.roughness * 2.0);
        let reflect_dir = reflect(ray, normal);
        let ior_water_to_mat = IOR_WATER / mat.ior;
        let refract_dir_in = refract(ray, normal, ior_water_to_mat);
        
        var refract_color = mat.base_color * 0.3;
        var reflect_color = vec3<f32>(0.0);
        let is_transparent = mat.absorption.x < 0.5;
        
        if (length(refract_dir_in) > 0.001 && is_transparent) {
            let local_refract_dir = rotate_vector(refract_dir_in, inv_rotation);
            let t_exit = get_exit_dist_shape(local_p, local_refract_dir, shape_type, radius);
            if (t_exit > 0.001) {
                let exit_point = hit + refract_dir_in * t_exit;
                let local_exit = local_p + local_refract_dir * t_exit;
                let dx2 = get_shape_dist(local_exit + vec3<f32>(e,0.0,0.0), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(e,0.0,0.0), shape_type, radius);
                let dy2 = get_shape_dist(local_exit + vec3<f32>(0.0,e,0.0), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(0.0,e,0.0), shape_type, radius);
                let dz2 = get_shape_dist(local_exit + vec3<f32>(0.0,0.0,e), shape_type, radius) - get_shape_dist(local_exit - vec3<f32>(0.0,0.0,e), shape_type, radius);
                let local_exit_normal = normalize(vec3<f32>(dx2, dy2, dz2));
                let exit_normal = rotate_vector(local_exit_normal, rotation);
                let ior_mat_to_water = mat.ior / IOR_WATER;
                let refract_dir_out = refract(refract_dir_in, -exit_normal, ior_mat_to_water);
                
                if (length(refract_dir_out) > 0.001) {
                    var t_bg = 1e30;
                    if (uniforms.pool_shape == 2) {
                        let res = intersect_cylinder_walls(exit_point, refract_dir_out, pool_size.x, -pool_height, wall_height);
                        if (res.y > 0.0) { t_bg = res.y; }
                    } else if (uniforms.pool_shape == 1) {
                        let s_m = pool_size * 0.7;
                        let t = intersect_frustum(exit_point, refract_dir_out, -pool_height, wall_height, s_m, pool_size);
                        t_bg = t.y;
                    } else {
                        let cube_min = vec3<f32>(-pool_size.x, -pool_height, -pool_size.y);
                        let cube_max = vec3<f32>(pool_size.x, wall_height, pool_size.y);
                        let t = intersect_cube(exit_point, refract_dir_out, cube_min, cube_max);
                        t_bg = t.y;
                    }
                    let bg_hit = exit_point + refract_dir_out * t_bg;
                    let coord = bg_hit.xz / (pool_size * 2.0) + 0.5;
                    let water_info = textureSample(water_texture, water_sampler, coord);
                    let caustic = textureSample(caustic_texture, caustic_sampler, coord);
                    let tiling = 2.0;
                    var tile_coord: vec2<f32>;
                    if (uniforms.pool_shape == 2) {
                         let r = length(bg_hit.xz);
                         if (r > pool_size.x - 0.05) {
                             let angle = atan2(bg_hit.z, bg_hit.x);
                             let u = angle / (2.0 * 3.14159) + 0.5;
                             tile_coord = vec2<f32>(u * 8.0, bg_hit.y * tiling * 0.5 + 0.5);
                         } else { tile_coord = bg_hit.xz * tiling * 0.5 + 0.5; }
                    } else {
                        if (bg_hit.y < -pool_height + 0.01) {
                             tile_coord = bg_hit.xz * tiling * 0.5 + 0.5;
                        } else {
                             if abs(bg_hit.x) > abs(bg_hit.z) { tile_coord = vec2<f32>(bg_hit.z, bg_hit.y) * tiling * 0.5 + 0.5; }
                             else { tile_coord = vec2<f32>(bg_hit.x, bg_hit.y) * tiling * 0.5 + 0.5; }
                        }
                    }
                    let tile_color = textureSample(tile_texture, tile_sampler, tile_coord).rgb;
                    refract_color = get_wall_color(bg_hit, uniforms, water_info, caustic, tile_color);
                    refract_color *= exp(-mat.absorption * t_exit * 6.0);
                    refract_color *= mat.base_color;
                }
            }
        } else if (!is_transparent) {
            let diffuse = max(0.0, dot(normal, light));
            let ambient = 0.35;
            refract_color = mat.base_color * (ambient + diffuse * 0.65) * uniforms.light_color.rgb;
        }
        
        var t_refl = 1e30;
        if (uniforms.pool_shape == 2) {
            let res = intersect_cylinder_walls(hit, reflect_dir, pool_size.x, -pool_height, wall_height);
            if (res.y > 0.0) { t_refl = res.y; }
        } else if (uniforms.pool_shape == 1) {
            let s_m = pool_size * 0.7;
            let t = intersect_frustum(hit, reflect_dir, -pool_height, wall_height, s_m, pool_size);
            t_refl = t.y;
        } else {
            let cube_min = vec3<f32>(-pool_size.x, -pool_height, -pool_size.y);
            let cube_max = vec3<f32>(pool_size.x, wall_height, pool_size.y);
            let t = intersect_cube(hit, reflect_dir, cube_min, cube_max);
            t_refl = t.y;
        }
        let refl_hit = hit + reflect_dir * t_refl;
        let refl_coord = refl_hit.xz / (pool_size * 2.0) + 0.5;
        let refl_water_info = textureSample(water_texture, water_sampler, refl_coord);
        let refl_caustic = textureSample(caustic_texture, caustic_sampler, refl_coord);
        let tiling = 2.0;
        var refl_tile_coord: vec2<f32>;
        if (uniforms.pool_shape == 2) {
             let r = length(refl_hit.xz);
             if (r > pool_size.x - 0.05) {
                 let angle = atan2(refl_hit.z, refl_hit.x);
                 let u = angle / (2.0 * 3.14159) + 0.5;
                 refl_tile_coord = vec2<f32>(u * 8.0, refl_hit.y * tiling * 0.5 + 0.5);
             } else {
                 refl_tile_coord = refl_hit.xz * tiling * 0.5 + 0.5;
             }
        } else {
            if (refl_hit.y < -pool_height + 0.01) { refl_tile_coord = refl_hit.xz * tiling * 0.5 + 0.5; }
            else {
                if abs(refl_hit.x) > abs(refl_hit.z) { refl_tile_coord = vec2<f32>(refl_hit.z, refl_hit.y) * tiling * 0.5 + 0.5; }
                else { refl_tile_coord = vec2<f32>(refl_hit.x, refl_hit.y) * tiling * 0.5 + 0.5; }
            }
        }
        let refl_tile_color = textureSample(tile_texture, tile_sampler, refl_tile_coord).rgb;
        reflect_color = get_wall_color(refl_hit, uniforms, refl_water_info, refl_caustic, refl_tile_color);
        if (uniforms.texture_type == 2) { reflect_color *= mat.base_color; }
        let spec_power = 30.0 + (1.0 - mat.roughness) * 150.0;
        let spec = pow(max(0.0, dot(reflect_dir, light)), spec_power) * mat.specularity;
        color = mix(refract_color, reflect_color, fresnel) + uniforms.light_color.rgb * spec * 0.25;
        
    } else if ray.y < 0.0 {
        let coord = hit.xz / (pool_size * 2.0) + 0.5;
        let water_info = textureSample(water_texture, water_sampler, coord);
        let caustic = textureSample(caustic_texture, caustic_sampler, coord);
        let tiling = 2.0;
        var tile_coord: vec2<f32>;
        if (uniforms.pool_shape == 2) {
             let r = length(hit.xz);
             if (r > pool_size.x - 0.05) {
                 let angle = atan2(hit.z, hit.x);
                 let u = angle / (2.0 * 3.14159) + 0.5;
                 tile_coord = vec2<f32>(u * 8.0, hit.y * tiling * 0.5 + 0.5);
             } else { tile_coord = hit.xz * tiling * 0.5 + 0.5; }
        } else {
            if (hit.y < -pool_height + 0.01) { tile_coord = hit.xz * tiling * 0.5 + 0.5; }
            else {
                if abs(hit.x) > abs(hit.z) { tile_coord = vec2<f32>(hit.z, hit.y) * tiling * 0.5 + 0.5; }
                else { tile_coord = vec2<f32>(hit.x, hit.y) * tiling * 0.5 + 0.5; }
            }
        }
        let tile_color = textureSample(tile_texture, tile_sampler, tile_coord).rgb;
        color = get_wall_color(hit, uniforms, water_info, caustic, tile_color);
    } else {
        if hit.y < wall_height - 0.001 {
            let coord = hit.xz / (pool_size * 2.0) + 0.5;
            let water_info = textureSample(water_texture, water_sampler, coord);
            let caustic = textureSample(caustic_texture, caustic_sampler, coord);
            let tiling = 2.0;
            var tile_coord: vec2<f32>;
            if (uniforms.pool_shape == 2) {
                 let r = length(hit.xz);
                 if (r > pool_size.x - 0.05) {
                     let angle = atan2(hit.z, hit.x);
                     let u = angle / (2.0 * 3.14159) + 0.5;
                     tile_coord = vec2<f32>(u * 8.0, hit.y * tiling * 0.5 + 0.5);
                 } else { tile_coord = hit.xz * tiling * 0.5 + 0.5; }
            } else {
                if abs(hit.x) > abs(hit.z) { tile_coord = vec2<f32>(hit.z, hit.y) * tiling * 0.5 + 0.5; }
                else { tile_coord = vec2<f32>(hit.x, hit.y) * tiling * 0.5 + 0.5; }
            }
            let tile_color = textureSample(tile_texture, tile_sampler, tile_coord).rgb;
            color = get_wall_color(hit, uniforms, water_info, caustic, tile_color);
        } else {
            let sky_uv = ray.xz * 0.5 + 0.5;
            color = textureSample(sky_texture, sky_sampler, sky_uv).rgb;
            color += vec3<f32>(pow(max(0.0, dot(light, ray)), 1000.0)) * vec3<f32>(8.0, 6.0, 4.0);
        }
    }
    if ray.y < 0.0 { color *= water_color; }
    return color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let position = in.world_pos;
    let pool_size = uniforms.pool_size;
    if (uniforms.pool_shape == 2) {
        if (length(position.xz) > pool_size.x) { discard; }
    } else if (uniforms.pool_shape == 1) {
        let h_total = uniforms.wall_height + uniforms.pool_height;
        let t = uniforms.pool_height / h_total;
        let scale = mix(0.7, 1.0, t);
        if (abs(position.x) > pool_size.x * scale || abs(position.z) > pool_size.y * scale) { discard; }
    }
    var coord = position.xz / (pool_size * 2.0) + 0.5;
    var info = textureSample(water_texture, water_sampler, coord);
    for (var i = 0; i < 2; i++) {
        coord += info.ba * 0.008;  // Slightly stronger offset to compensate for fewer iterations
        info = textureSample(water_texture, water_sampler, coord);
    }
    let normal = vec3<f32>(info.b, sqrt(1.0 - dot(info.ba, info.ba)), info.a);
    let incoming_ray = normalize(position - camera.eye.xyz);
    var reflected_ray: vec3<f32>;
    var refracted_ray: vec3<f32>;
    var fresnel: f32;
    if dot(incoming_ray, normal) < 0.0 {
        reflected_ray = reflect(incoming_ray, normal);
        refracted_ray = refract(incoming_ray, normal, IOR_AIR / IOR_WATER);
        fresnel = mix(0.25, 1.0, pow(1.0 - dot(normal, -incoming_ray), 3.0));
    } else {
        let flipped_normal = -normal;
        reflected_ray = reflect(incoming_ray, flipped_normal);
        refracted_ray = refract(incoming_ray, flipped_normal, IOR_WATER / IOR_AIR);
        fresnel = mix(0.25, 1.0, pow(1.0 - dot(flipped_normal, -incoming_ray), 3.0));
    }
    
    var reflected_color: vec3<f32>;
    var refracted_color: vec3<f32> = vec3<f32>(0.0);
    
    if (uniforms.enable_raytracing == 1) {
        reflected_color = get_surface_ray_color(position, reflected_ray, ABOVE_WATER_COLOR);
        if length(refracted_ray) > 0.001 { refracted_color = get_surface_ray_color(position, refracted_ray, ABOVE_WATER_COLOR); }
        else { fresnel = 1.0; }
    } else {
        // Simple fallback
        let sky_uv = reflected_ray.xz * 0.5 + 0.5;
        reflected_color = textureSample(sky_texture, sky_sampler, sky_uv).rgb;
        // Simple refraction
        let tile_uv = position.xz / (pool_size * 2.0) + 0.5;
        refracted_color = textureSample(tile_texture, tile_sampler, tile_uv).rgb * UNDERWATER_COLOR;
    }
    
    return vec4<f32>(mix(refracted_color, reflected_color, fresnel), 1.0);
}
