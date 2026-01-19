// Index of refraction constants
const IOR_AIR: f32 = 1.0;
const IOR_WATER: f32 = 1.333;

// Water colors
const ABOVE_WATER_COLOR: vec3<f32> = vec3<f32>(0.6, 0.9, 1.0);
const UNDERWATER_COLOR: vec3<f32> = vec3<f32>(0.4, 0.9, 1.0);

// Camera uniforms structure
struct CameraUniforms {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

// Common uniforms structure
struct CommonUniforms {
    pool_height: f32,
    wall_height: f32,
    pool_size: vec2<f32>,
    light_dir: vec4<f32>,
    sphere_radius: f32,
    time: f32,
    shape_type: i32,
    texture_type: i32,
    object_count: i32,
    pool_shape: i32,
    enable_gi: i32,
    enable_raytracing: i32,
    light_color: vec4<f32>,
}

@group(0) @binding(1) var<uniform> uniforms: CommonUniforms;
@group(0) @binding(2) var<storage, read> sphere_centers: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> sphere_rotations: array<vec4<f32>>;

fn rotate_vector(v: vec3<f32>, q: vec4<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

fn sdSphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sdTorus(p: vec3<f32>, t_param: vec2<f32>) -> f32 {
    let q = vec2<f32>(length(p.xz) - t_param.x, p.y);
    return length(q) - t_param.y;
}

fn get_shape_dist(p: vec3<f32>, shape_type: i32, radius: f32) -> f32 {
    if (shape_type == 0) { // Sphere
        return sdSphere(p, radius);
    } else if (shape_type == 1) { // Torus
        return sdTorus(p, vec2<f32>(0.7 * radius, 0.3 * radius));
    } else if (shape_type == 2) { // Tetrahedron
        return (max(abs(p.x+p.y)-p.z, abs(p.x-p.y)+p.z) - 1.0 * radius) / sqrt(3.0);
    } else { // Cube
        return sdBox(p, vec3<f32>(0.577 * radius));
    }
}

// Ray-primitive intersections
fn intersect_sphere(origin: vec3<f32>, ray: vec3<f32>, center: vec3<f32>, radius: f32) -> f32 {
    let to_sphere = origin - center;
    let b = dot(to_sphere, ray);
    let c = dot(to_sphere, to_sphere) - radius * radius;
    let d = b * b - c;
    if (d > 0.0) {
        let t = -b - sqrt(d);
        if (t > 0.0) { return t; }
    }
    return -1.0;
}

fn intersect_cube(origin: vec3<f32>, ray: vec3<f32>, cube_min: vec3<f32>, cube_max: vec3<f32>) -> vec2<f32> {
    let t_min = (cube_min - origin) / ray;
    let t_max = (cube_max - origin) / ray;
    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);
    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);
    return vec2<f32>(t_near, t_far);
}

fn intersect_cylinder_walls(origin: vec3<f32>, ray: vec3<f32>, radius: f32, y_min: f32, y_max: f32) -> vec2<f32> {
    let a = ray.x * ray.x + ray.z * ray.z;
    let b = 2.0 * (origin.x * ray.x + origin.z * ray.z);
    let c = origin.x * origin.x + origin.z * origin.z - radius * radius;
    
    let discriminant = b * b - 4.0 * a * c;
    
    var t_near = -1.0;
    var t_far = -1.0;
    
    if (discriminant >= 0.0) {
        let t1 = (-b - sqrt(discriminant)) / (2.0 * a);
        let t2 = (-b + sqrt(discriminant)) / (2.0 * a);
        
        // Check height bounds for t1
        let y1 = origin.y + ray.y * t1;
        if (y1 >= y_min && y1 <= y_max) {
            t_near = t1;
        }
        
        // Check height bounds for t2
        let y2 = origin.y + ray.y * t2;
        if (y2 >= y_min && y2 <= y_max) {
            if (t_near < 0.0) { t_near = t2; }
            else { t_far = t2; }
        }
    }
    
    // Also check floor and ceiling caps
    if (abs(ray.y) > 1e-6) {
        let t_floor = (y_min - origin.y) / ray.y;
        let p_floor = origin + ray * t_floor;
        if (length(p_floor.xz) <= radius) {
            if (t_near < 0.0 || (t_floor < t_near && t_floor > 0.0)) {
                t_far = t_near;
                t_near = t_floor;
            } else if (t_far < 0.0 || t_floor < t_far) {
                t_far = t_floor;
            }
        }
        
        let t_ceil = (y_max - origin.y) / ray.y;
        let p_ceil = origin + ray * t_ceil;
        if (length(p_ceil.xz) <= radius) {
            if (t_near < 0.0 || (t_ceil < t_near && t_ceil > 0.0)) {
                t_far = t_near;
                t_near = t_ceil;
            } else if (t_far < 0.0 || t_ceil < t_far) {
                t_far = t_ceil;
            }
        }
    }
    
    return vec2<f32>(t_near, t_far);
}

fn intersect_frustum(origin: vec3<f32>, ray: vec3<f32>, y_min: f32, y_max: f32, s_min: vec2<f32>, s_max: vec2<f32>) -> vec2<f32> {
    let slope = (s_max - s_min) / (y_max - y_min);
    let base = s_min - slope * y_min;
    
    var t_enter = -1e30;
    var t_exit = 1e30;
    
    // Plane 1 (Right)
    {
        let n = vec3<f32>(1.0, -slope.x, 0.0);
        let d = base.x;
        let denom = dot(n, ray);
        let num = d - dot(n, origin);
        if (abs(denom) > 1e-6) {
            let t = num / denom;
            if (denom > 0.0) { t_exit = min(t_exit, t); }
            else { t_enter = max(t_enter, t); }
        } else {
             if (dot(n, origin) > d) { return vec2<f32>(-1.0, -1.0); }
        }
    }
    // Plane 2 (Left)
    {
        let n = vec3<f32>(1.0, slope.x, 0.0);
        let d = -base.x;
        let denom = dot(n, ray);
        let num = d - dot(n, origin);
        if (abs(denom) > 1e-6) {
            let t = num / denom;
            if (denom < 0.0) { t_exit = min(t_exit, t); }
            else { t_enter = max(t_enter, t); }
        } else {
            if (dot(n, origin) < d) { return vec2<f32>(-1.0, -1.0); }
        }
    }
    // Plane 3 (Back Z+)
    {
        let n = vec3<f32>(0.0, -slope.y, 1.0);
        let d = base.y;
        let denom = dot(n, ray);
        let num = d - dot(n, origin);
        if (abs(denom) > 1e-6) {
            let t = num / denom;
            if (denom > 0.0) { t_exit = min(t_exit, t); }
            else { t_enter = max(t_enter, t); }
        } else {
             if (dot(n, origin) > d) { return vec2<f32>(-1.0, -1.0); }
        }
    }
    // Plane 4 (Front Z-)
    {
        let n = vec3<f32>(0.0, slope.y, 1.0);
        let d = -base.y;
        let denom = dot(n, ray);
        let num = d - dot(n, origin);
        if (abs(denom) > 1e-6) {
            let t = num / denom;
            if (denom < 0.0) { t_exit = min(t_exit, t); }
            else { t_enter = max(t_enter, t); }
        } else {
            if (dot(n, origin) < d) { return vec2<f32>(-1.0, -1.0); }
        }
    }
    
    // Y Slabs
    if (abs(ray.y) > 1e-6) {
        let t1 = (y_min - origin.y) / ray.y;
        let t2 = (y_max - origin.y) / ray.y;
        t_enter = max(t_enter, min(t1, t2));
        t_exit = min(t_exit, max(t1, t2));
    } else {
        if (origin.y < y_min || origin.y > y_max) { return vec2<f32>(-1.0, -1.0); }
    }
    
    if (t_enter < t_exit && t_exit > 0.0) {
        return vec2<f32>(max(0.0, t_enter), t_exit);
    }
    return vec2<f32>(-1.0, -1.0);
}

fn intersect_single_shape(origin: vec3<f32>, ray: vec3<f32>, center: vec3<f32>, radius: f32, shape_type: i32, rotation: vec4<f32>) -> f32 {
    let bound_r = radius * 2.0;
    var t = 0.0;
    let dist_to_center = length(origin - center);
    if (dist_to_center > bound_r) {
        let t_sphere = intersect_sphere(origin, ray, center, bound_r);
        if (t_sphere < 0.0) { return -1.0; }
        t = max(0.0, t_sphere - 0.01);
    }
    if (shape_type == 0) { 
        if (dist_to_center < radius) { return 0.0; } 
        return intersect_sphere(origin, ray, center, radius); 
    }
    let inv_rotation = vec4<f32>(-rotation.xyz, rotation.w);
    let local_origin = rotate_vector(origin - center, inv_rotation);
    let local_ray = rotate_vector(ray, inv_rotation);
    if (get_shape_dist(local_origin, shape_type, radius) < 0.0) {
        return 0.0;
    }
    for (var i=0; i<64; i++) {
        let p = local_origin + local_ray * t;
        let d = get_shape_dist(p, shape_type, radius);
        if (d < 0.0005) { return t; }
        t += d;
        if (t > radius * 4.0) { return -1.0; }
    }
    return -1.0;
}

fn intersect_shape_any(origin: vec3<f32>, ray: vec3<f32>, uniforms: CommonUniforms) -> vec2<f32> {
    var best_t = 1e30;
    var best_idx = -1.0;
    
    for (var i: i32 = 0; i < uniforms.object_count; i++) {
        let t = intersect_single_shape(origin, ray, sphere_centers[i].xyz, uniforms.sphere_radius, uniforms.shape_type, sphere_rotations[i]);
        if (t > 0.0 && t < best_t) {
            best_t = t;
            best_idx = f32(i);
        }
    }

    if (best_idx < -0.5) { return vec2<f32>(-1.0, -1.0); }
    return vec2<f32>(best_t, best_idx);
}

fn get_exit_dist_shape(origin: vec3<f32>, ray: vec3<f32>, shape_type: i32, radius: f32) -> f32 {
    var t = 0.001;
    for (var i = 0; i < 64; i++) {
        let p = origin + ray * t;
        let d = get_shape_dist(p, shape_type, radius);
        if (d > -0.0005) { return t; }
        t += abs(d);
        if (t > radius * 4.0) { break; }
    }
    return t;
}

struct MaterialProps {
    ior: f32,
    base_color: vec3<f32>,
    absorption: vec3<f32>,
    specularity: f32,
    roughness: f32,
}

fn get_material_props(texture_type: i32) -> MaterialProps {
    var mat: MaterialProps;
    if (texture_type == 0) { // Glass
        mat.ior = 1.5;
        mat.base_color = vec3<f32>(0.9, 0.95, 1.0);
        mat.absorption = vec3<f32>(0.05, 0.02, 0.0);
        mat.specularity = 1.0;
        mat.roughness = 0.0;
    } else if (texture_type == 1) { // Wood
        mat.ior = 1.3;
        mat.base_color = vec3<f32>(0.4, 0.25, 0.1);
        mat.absorption = vec3<f32>(1.0, 1.0, 1.0);
        mat.specularity = 0.1;
        mat.roughness = 0.8;
    } else if (texture_type == 2) { // Steel
        mat.ior = 2.5;
        mat.base_color = vec3<f32>(0.8, 0.82, 0.85);
        mat.absorption = vec3<f32>(1.0, 1.0, 1.0);
        mat.specularity = 2.0;
        mat.roughness = 0.1;
    } else { // Ice
        mat.ior = 1.31;
        mat.base_color = vec3<f32>(0.85, 0.95, 1.0);
        mat.absorption = vec3<f32>(0.1, 0.05, 0.0);
        mat.specularity = 0.8;
        mat.roughness = 0.2;
    }
    return mat;
}

fn get_wall_color(point: vec3<f32>, uniforms: CommonUniforms, water_info: vec4<f32>, caustic_sample: vec4<f32>, tile_color: vec3<f32>) -> vec3<f32> {
    var scale = vec3<f32>(0.5);
    let pool_size = uniforms.pool_size;
    let light = uniforms.light_dir.xyz;
    var normal: vec3<f32>;
    if (uniforms.pool_shape == 2) { // Tube
        let r = length(point.xz);
        if (r > pool_size.x - 0.05) { normal = normalize(vec3<f32>(-point.x, 0.0, -point.z)); }
        else { normal = vec3<f32>(0.0, 1.0, 0.0); }
    } else if (uniforms.pool_shape == 1) { // Frustum
        if (point.y < -uniforms.pool_height + 0.01) { normal = vec3<f32>(0.0, 1.0, 0.0); }
        else {
             if abs(point.x) > abs(point.z) { normal = vec3<f32>(-sign(point.x), 0.2, 0.0); }
             else { normal = vec3<f32>(0.0, 0.2, -sign(point.z)); }
             normal = normalize(normal);
        }
    } else { // Cube/Cuboid
        if abs(point.x) > pool_size.x - 0.01 { normal = vec3<f32>(-sign(point.x), 0.0, 0.0); }
        else if abs(point.z) > pool_size.y - 0.01 { normal = vec3<f32>(0.0, 0.0, -sign(point.z)); }
        else { normal = vec3<f32>(0.0, 1.0, 0.0); }
    }
    let refracted_light = -refract(-light, vec3<f32>(0.0, 1.0, 0.0), IOR_AIR / IOR_WATER);
    let diffuse = max(0.0, dot(refracted_light, normal));
    let lit_diffuse = diffuse * uniforms.light_color.rgb;
    var shadow = 1.0;
    let res = intersect_shape_any(point, refracted_light, uniforms);
    if (res.x > 0.0) { shadow = 0.2; }
    if point.y < water_info.r { scale += lit_diffuse * caustic_sample.r * 3.0 * caustic_sample.g * shadow; }
    else { scale += lit_diffuse * shadow * 0.5; }
    return tile_color * scale;
}

fn get_sphere_color(point: vec3<f32>, uniforms: CommonUniforms, water_info: vec4<f32>, caustic_sample: vec4<f32>, shape_idx: i32) -> vec3<f32> {
    let center = sphere_centers[shape_idx].xyz;
    let rotation = sphere_rotations[shape_idx];
    
    let radius = uniforms.sphere_radius;
    let light = uniforms.light_dir.xyz;
    let inv_rotation = vec4<f32>(-rotation.xyz, rotation.w);
    let local_p = rotate_vector(point - center, inv_rotation);
    let shape_type = uniforms.shape_type;
    let e = 0.001;
    let dx = get_shape_dist(local_p + vec3<f32>(e,0.0,0.0), shape_type, radius) - get_shape_dist(local_p - vec3<f32>(e,0.0,0.0), shape_type, radius);
    let dy = get_shape_dist(local_p + vec3<f32>(0.0,e,0.0), shape_type, radius) - get_shape_dist(local_p - vec3<f32>(0.0,e,0.0), shape_type, radius);
    let dz = get_shape_dist(local_p + vec3<f32>(0.0,0.0,e), shape_type, radius) - get_shape_dist(local_p - vec3<f32>(0.0,0.0,e), shape_type, radius);
    let local_normal = normalize(vec3<f32>(dx, dy, dz));
    let normal = rotate_vector(local_normal, rotation);
    let refracted_light = refract(-light, vec3<f32>(0.0, 1.0, 0.0), IOR_AIR / IOR_WATER);
    var diffuse = max(0.0, dot(-refracted_light, normal)) * 0.5;
    if point.y < water_info.r { diffuse *= caustic_sample.r * 4.0; }
    return vec3<f32>(0.5) * diffuse * uniforms.light_color.rgb + vec3<f32>(0.2) * uniforms.light_color.rgb;
}
