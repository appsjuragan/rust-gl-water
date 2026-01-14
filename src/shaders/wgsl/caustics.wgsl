struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) old_pos: vec3<f32>,
    @location(1) new_pos: vec3<f32>,
    @location(2) ray: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: CommonUniforms;
@group(0) @binding(1) var water_texture: texture_2d<f32>;
@group(0) @binding(2) var water_sampler: sampler;

fn project(origin: vec3<f32>, ray: vec3<f32>, refracted_light: vec3<f32>, pool_size: vec2<f32>, pool_height: f32, wall_height: f32, pool_shape: i32) -> vec3<f32> {
    var t_cube: vec2<f32>;
    if (pool_shape == 2) {
        t_cube = intersect_cylinder_walls(origin, ray, pool_size.x, -pool_height, wall_height);
    } else if (pool_shape == 1) {
        t_cube = intersect_frustum(origin, ray, -pool_height, wall_height, pool_size * 0.7, pool_size);
    } else {
        let cube_min = vec3<f32>(-pool_size.x, -pool_height, -pool_size.y);
        let cube_max = vec3<f32>(pool_size.x, wall_height, pool_size.y);
        t_cube = intersect_cube(origin, ray, cube_min, cube_max);
    }
    let hit = origin + ray * t_cube.y;
    let t_plane = (-hit.y - 1.0) / refracted_light.y;
    return hit + refracted_light * t_plane;
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let info = textureSampleLevel(water_texture, water_sampler, in.uv, 0.0);
    let normal_ba = info.ba * 0.5;
    let normal = vec3<f32>(normal_ba.x, sqrt(1.0 - dot(normal_ba, normal_ba)), normal_ba.y);
    let light = uniforms.light_dir.xyz;
    let pool_size = uniforms.pool_size;
    let pool_height = uniforms.pool_height;
    let wall_height = uniforms.wall_height;
    let refracted_light = refract(-light, vec3<f32>(0.0, 1.0, 0.0), IOR_AIR / IOR_WATER);
    out.ray = refract(-light, normal, IOR_AIR / IOR_WATER);
    var raw_pos = vec3<f32>(in.uv.x * 2.0 - 1.0, 0.0, in.uv.y * 2.0 - 1.0);
    raw_pos.x *= pool_size.x;
    raw_pos.z *= pool_size.y;
    out.old_pos = project(raw_pos, refracted_light, refracted_light, pool_size, pool_height, wall_height, uniforms.pool_shape);
    out.new_pos = project(raw_pos + vec3<f32>(0.0, info.r, 0.0), out.ray, refracted_light, pool_size, pool_height, wall_height, uniforms.pool_shape);
    let proj = 0.75 * (out.new_pos.xz + refracted_light.xz / refracted_light.y) / pool_size;
    out.position = vec4<f32>(proj, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let old_area = length(dpdx(in.old_pos)) * length(dpdy(in.old_pos));
    let new_area = length(dpdx(in.new_pos)) * length(dpdy(in.new_pos));
    var caustic_intensity = old_area / new_area * 0.2;
    var shadow = 1.0;
    let light = uniforms.light_dir.xyz;
    let refracted_light = refract(-light, vec3<f32>(0.0, 1.0, 0.0), IOR_AIR / IOR_WATER);
    let pool_size = uniforms.pool_size;
    let pool_height = uniforms.pool_height;
    let wall_height = uniforms.wall_height;
    var t: vec2<f32>;
    if (uniforms.pool_shape == 2) {
        t = intersect_cylinder_walls(in.new_pos, -refracted_light, pool_size.x, -pool_height, wall_height);
    } else if (uniforms.pool_shape == 1) {
        t = intersect_frustum(in.new_pos, -refracted_light, -pool_height, wall_height, pool_size * 0.7, pool_size);
    } else {
        let cube_min = vec3<f32>(-pool_size.x, -pool_height, -pool_size.y);
        let cube_max = vec3<f32>(pool_size.x, wall_height, pool_size.y);
        t = intersect_cube(in.new_pos, -refracted_light, cube_min, cube_max);
    }
    caustic_intensity *= 1.0 / (1.0 + exp(-200.0 / (1.0 + 10.0 * (t.y - t.x)) * (in.new_pos.y - refracted_light.y * t.y - wall_height)));
    return vec4<f32>(caustic_intensity, shadow, 0.0, 1.0);
}
