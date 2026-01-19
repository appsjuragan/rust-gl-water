struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

@group(1) @binding(0) var water_texture: texture_2d<f32>;
@group(1) @binding(1) var water_sampler: sampler;
@group(1) @binding(2) var tile_texture: texture_2d<f32>;
@group(1) @binding(3) var tile_sampler: sampler;
@group(1) @binding(4) var caustic_texture: texture_2d<f32>;
@group(1) @binding(5) var caustic_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
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
    out.normal = in.normal;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let position = in.world_pos;
    let wall_height = uniforms.wall_height;
    let pool_size = uniforms.pool_size;
    
    // Discard above-water wall tops
    if position.y > wall_height - 0.001 {
        discard;
    }
    
    let coord = position.xz / (pool_size * 2.0) + 0.5;
    let water_info = textureSample(water_texture, water_sampler, coord);
    let caustic = textureSample(caustic_texture, caustic_sampler, coord);
    
    // Calculate tile coordinates based on shape and normal
    var tile_coord: vec2<f32>;
    let n = normalize(in.normal);
    
    // Tiling factor - higher = smaller tiles
    let tiling = 2.0;
    
    // Check if floor (up pointing normal)
    if (dot(n, vec3<f32>(0.0, 1.0, 0.0)) > 0.9) {
        tile_coord = position.xz * tiling * 0.5 + 0.5;
    } else {
        // Walls
        if (uniforms.pool_shape == 2) { // Cylinder
            // Use angle for U, y for V
            let angle = atan2(position.z, position.x);
            let u = angle / (2.0 * 3.14159) + 0.5;
            // Scale u to repeat texture around cylinder
            tile_coord = vec2<f32>(u * 8.0, position.y * tiling * 0.5 + 0.5);
        } else { // Cube/Cuboid/Frustum
            if (abs(n.x) > 0.5) {
                // Normal along X (Left/Right walls). Wall length along Z.
                tile_coord = vec2<f32>(position.z, position.y) * tiling * 0.5 + 0.5;
            } else {
                // Normal along Z (Front/Back walls). Wall length along X.
                tile_coord = vec2<f32>(position.x, position.y) * tiling * 0.5 + 0.5;
            }
        }
    }
    
    let tile_color = textureSample(tile_texture, tile_sampler, tile_coord).rgb;
    var color = get_wall_color(position, uniforms, water_info, caustic, tile_color);
    
    // Underwater tint
    if position.y < water_info.r {
        color *= UNDERWATER_COLOR * 1.2;
    }
    
    return vec4<f32>(color, 1.0);
}
