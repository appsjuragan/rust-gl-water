//! Standard collider implementation using bounding spheres

use crate::core::physics_trait::{Collider, CollisionInfo, PhysicsState};
use crate::core::shape::{Shape, ShapeParams};
use glam::Vec3;

/// Standard bounding-sphere based collider
#[allow(dead_code)]
pub struct BoundingSphereCollider;

impl BoundingSphereCollider {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

impl Default for BoundingSphereCollider {
    fn default() -> Self {
        Self::new()
    }
}

impl Collider for BoundingSphereCollider {
    fn collide_with_box(
        &self,
        state: &mut PhysicsState,
        half_width: f32,
        half_length: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let (_, radius) = shape.bounding_sphere(shape_params);

        // X boundaries
        if state.position.x < -half_width + radius {
            state.position.x = -half_width + radius;
            if state.velocity.x < 0.0 {
                state.velocity.x = -state.velocity.x * 0.5;
            }
        } else if state.position.x > half_width - radius {
            state.position.x = half_width - radius;
            if state.velocity.x > 0.0 {
                state.velocity.x = -state.velocity.x * 0.5;
            }
        }

        // Z boundaries
        if state.position.z < -half_length + radius {
            state.position.z = -half_length + radius;
            if state.velocity.z < 0.0 {
                state.velocity.z = -state.velocity.z * 0.5;
            }
        } else if state.position.z > half_length - radius {
            state.position.z = half_length - radius;
            if state.velocity.z > 0.0 {
                state.velocity.z = -state.velocity.z * 0.5;
            }
        }
    }

    fn collide_with_cylinder(
        &self,
        state: &mut PhysicsState,
        radius: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let (_, obj_radius) = shape.bounding_sphere(shape_params);
        let max_radius = radius - obj_radius;

        let dist_xz = (state.position.x * state.position.x + state.position.z * state.position.z).sqrt();

        if dist_xz > max_radius {
            let normal = Vec3::new(state.position.x, 0.0, state.position.z).normalize();
            state.position = normal * max_radius + Vec3::new(0.0, state.position.y, 0.0);

            let vel_normal = state.velocity.dot(normal);
            if vel_normal > 0.0 {
                state.velocity -= normal * vel_normal * 1.5;
            }
        }
    }

    fn collide_with_frustum(
        &self,
        state: &mut PhysicsState,
        top_scale: f32, // half-width at top
        bottom_scale: f32, // half-width at bottom
        y_min: f32,
        y_max: f32,
        shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let (_, radius) = shape.bounding_sphere(shape_params);
        let dy = y_max - y_min;
        let dx = top_scale - bottom_scale;
        
        // Calculate normals for the 4 sloped planes
        // The normals should point INWARD to the pool (and slightly UP because the pool widens upward)
        
        // 1. Right Wall (+X): Point P=(bottom_scale, y_min, 0). Normal n points (-X, +Y)
        // Vector along wall (in XY): (dx, dy). Orthogonal (-dy, dx).
        let n_right = Vec3::new(-dy, dx, 0.0).normalize();
        let d_right = Vec3::new(bottom_scale, y_min, 0.0).dot(n_right);

        // 2. Left Wall (-X): Point P=(-bottom_scale, y_min, 0). Normal n points (+X, +Y)
        let n_left = Vec3::new(dy, dx, 0.0).normalize();
        let d_left = Vec3::new(-bottom_scale, y_min, 0.0).dot(n_left);

        // 3. Front Wall (+Z): Point P=(0, y_min, bottom_scale). Normal n points (-Z, +Y)
        // Vector along wall (in ZY): (dx, dy). Orthogonal (-dy, dx).
        let n_front = Vec3::new(0.0, dx, -dy).normalize();
        let d_front = Vec3::new(0.0, y_min, bottom_scale).dot(n_front);
        
        // 4. Back Wall (-Z): Point P=(0, y_min, -bottom_scale). Normal n points (+Z, +Y)
        let n_back = Vec3::new(0.0, dx, dy).normalize();
        let d_back = Vec3::new(0.0, y_min, -bottom_scale).dot(n_back);

        let planes = [
            (n_right, d_right),
            (n_left, d_left),
            (n_front, d_front),
            (n_back, d_back),
        ];

        for (normal, plane_d) in planes {
            let dist = state.position.dot(normal) - plane_d;
            
            // We are inside the simplified infinite pool cone if dist > radius. 
            // Collision if dist < radius (penetrating the exclusion zone)
            if dist < radius {
                let penetration = radius - dist;
                state.position += normal * penetration;

                // Velocity response
                let v_dot_n = state.velocity.dot(normal);
                if v_dot_n < 0.0 {
                    // Moving towards wall (outwards of pool)
                    let restitution = 0.2; // Damping
                    state.velocity -= normal * v_dot_n * (1.0 + restitution);
                    
                    // Apply friction to tangent velocity
                    let tangent = state.velocity - normal * state.velocity.dot(normal);
                    if tangent.length_squared() > 1e-6 {
                        state.velocity -= tangent * 0.1;
                    }
                }
            }
        }
    }

    fn check_object_collision(
        &self,
        state_a: &PhysicsState,
        state_b: &PhysicsState,
        shape_a: &dyn Shape,
        shape_b: &dyn Shape,
        params_a: &ShapeParams,
        params_b: &ShapeParams,
    ) -> Option<CollisionInfo> {
        let (_, radius_a) = shape_a.bounding_sphere(params_a);
        let (_, radius_b) = shape_b.bounding_sphere(params_b);

        let delta = state_b.position - state_a.position;
        let dist = delta.length();
        let broad_min_dist = radius_a + radius_b;

        // Broad phase: bounding spheres
        if dist > broad_min_dist {
            return None;
        }

        // Narrow phase: use SDF for more accuracy (especially for non-spheres)
        // We check the distance from B's center to A's surface using A's SDF
        let inv_rot_a = params_a.rotation.inverse();
        let local_pos_b = inv_rot_a * (state_b.position - state_a.position);
        let dist_to_surface_a = shape_a.sdf(local_pos_b, params_a);
        
        // Collision if B's radius penetrates A's surface
        let penetration = radius_b - dist_to_surface_a;

        if penetration > 0.0 && dist > 1e-6 {
            let normal = delta / dist;
            Some(CollisionInfo {
                normal,
                penetration_depth: penetration,
                contact_point: state_a.position + normal * (dist - radius_b + penetration * 0.5),
            })
        } else {
            None
        }
    }
}

/// Default buoyancy calculator implementation
#[allow(dead_code)]
pub struct StandardBuoyancy;

impl StandardBuoyancy {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

impl Default for StandardBuoyancy {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::core::physics_trait::BuoyancyCalculator for StandardBuoyancy {}

/// Tetrahedron-specific collider with vertex-based collision detection
#[allow(dead_code)]
pub struct TetrahedronCollider;

impl TetrahedronCollider {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
    
    /// Get the 4 vertices of a tetrahedron in world space
    fn get_vertices(center: Vec3, radius: f32, rotation: glam::Quat) -> [Vec3; 4] {
        let s = radius / 3.0f32.sqrt();
        let local_verts = [
            Vec3::new(s, s, s),
            Vec3::new(s, -s, -s),
            Vec3::new(-s, s, -s),
            Vec3::new(-s, -s, s),
        ];
        [
            center + rotation * local_verts[0],
            center + rotation * local_verts[1],
            center + rotation * local_verts[2],
            center + rotation * local_verts[3],
        ]
    }
    
    /// Get face normals in world space (pointing outward)
    fn get_face_normals(rotation: glam::Quat) -> [Vec3; 4] {
        // Face indices: (0,1,2), (0,2,3), (0,3,1), (1,3,2)
        let s = 1.0 / 3.0f32.sqrt();
        let local_normals = [
            Vec3::new(s, s, s).normalize(),      // Face 0-1-2
            Vec3::new(-s, s, -s).normalize(),    // Face 0-2-3
            Vec3::new(s, -s, -s).normalize(),    // Face 0-3-1
            Vec3::new(-s, -s, s).normalize(),    // Face 1-3-2
        ];
        [
            rotation * local_normals[0],
            rotation * local_normals[1],
            rotation * local_normals[2],
            rotation * local_normals[3],
        ]
    }
    
    /// Find the lowest vertex (for floor contact stability)
    fn find_lowest_vertex(verts: &[Vec3; 4]) -> (usize, f32) {
        let mut lowest_idx = 0;
        let mut lowest_y = verts[0].y;
        for (i, v) in verts.iter().enumerate() {
            if v.y < lowest_y {
                lowest_y = v.y;
                lowest_idx = i;
            }
        }
        (lowest_idx, lowest_y)
    }
}

impl Default for TetrahedronCollider {
    fn default() -> Self {
        Self::new()
    }
}

impl Collider for TetrahedronCollider {
    fn collide_with_box(
        &self,
        state: &mut PhysicsState,
        half_width: f32,
        half_length: f32,
        _shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let verts = Self::get_vertices(state.position, shape_params.radius, state.rotation);
        
        // Check each vertex against box walls
        for vert in &verts {
            // X walls
            if vert.x < -half_width {
                let penetration = -half_width - vert.x;
                state.position.x += penetration;
                if state.velocity.x < 0.0 {
                    state.velocity.x = -state.velocity.x * 0.3;
                }
            } else if vert.x > half_width {
                let penetration = vert.x - half_width;
                state.position.x -= penetration;
                if state.velocity.x > 0.0 {
                    state.velocity.x = -state.velocity.x * 0.3;
                }
            }
            
            // Z walls
            if vert.z < -half_length {
                let penetration = -half_length - vert.z;
                state.position.z += penetration;
                if state.velocity.z < 0.0 {
                    state.velocity.z = -state.velocity.z * 0.3;
                }
            } else if vert.z > half_length {
                let penetration = vert.z - half_length;
                state.position.z -= penetration;
                if state.velocity.z > 0.0 {
                    state.velocity.z = -state.velocity.z * 0.3;
                }
            }
        }
    }

    fn collide_with_cylinder(
        &self,
        state: &mut PhysicsState,
        radius: f32,
        _shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let verts = Self::get_vertices(state.position, shape_params.radius, state.rotation);
        
        for vert in &verts {
            let dist_xz = (vert.x * vert.x + vert.z * vert.z).sqrt();
            if dist_xz > radius {
                let penetration = dist_xz - radius;
                let normal = Vec3::new(vert.x, 0.0, vert.z).normalize();
                state.position -= normal * penetration;
                
                let vel_normal = state.velocity.dot(normal);
                if vel_normal > 0.0 {
                    state.velocity -= normal * vel_normal * 1.3;
                }
            }
        }
    }

    fn collide_with_frustum(
        &self,
        state: &mut PhysicsState,
        top_scale: f32,
        bottom_scale: f32,
        y_min: f32,
        y_max: f32,
        _shape: &dyn Shape,
        shape_params: &ShapeParams,
    ) {
        let verts = Self::get_vertices(state.position, shape_params.radius, state.rotation);
        let dy = y_max - y_min;
        let dx = top_scale - bottom_scale;
        
        // Define the 4 sloped planes
        let n_right = Vec3::new(-dy, dx, 0.0).normalize();
        let d_right = Vec3::new(bottom_scale, y_min, 0.0).dot(n_right);
        let n_left = Vec3::new(dy, dx, 0.0).normalize();
        let d_left = Vec3::new(-bottom_scale, y_min, 0.0).dot(n_left);
        let n_front = Vec3::new(0.0, dx, -dy).normalize();
        let d_front = Vec3::new(0.0, y_min, bottom_scale).dot(n_front);
        let n_back = Vec3::new(0.0, dx, dy).normalize();
        let d_back = Vec3::new(0.0, y_min, -bottom_scale).dot(n_back);

        let planes = [
            (n_right, d_right),
            (n_left, d_left),
            (n_front, d_front),
            (n_back, d_back),
        ];

        // Check each vertex against each plane
        for vert in &verts {
            for (normal, plane_d) in &planes {
                let dist = vert.dot(*normal) - *plane_d;
                if dist < 0.0 {
                    let penetration = -dist;
                    state.position += *normal * penetration;
                    
                    let v_dot_n = state.velocity.dot(*normal);
                    if v_dot_n < 0.0 {
                        state.velocity -= *normal * v_dot_n * 1.2;
                        // Tangent friction
                        let tangent = state.velocity - *normal * state.velocity.dot(*normal);
                        if tangent.length_squared() > 1e-6 {
                            state.velocity -= tangent * 0.15;
                        }
                    }
                }
            }
        }
    }

    fn check_object_collision(
        &self,
        state_a: &PhysicsState,
        state_b: &PhysicsState,
        _shape_a: &dyn Shape,
        _shape_b: &dyn Shape,
        params_a: &ShapeParams,
        params_b: &ShapeParams,
    ) -> Option<CollisionInfo> {
        // Get vertices of both tetrahedrons
        let verts_a = Self::get_vertices(state_a.position, params_a.radius, params_a.rotation);
        let verts_b = Self::get_vertices(state_b.position, params_b.radius, params_b.rotation);
        
        // Broad phase: bounding spheres
        let delta = state_b.position - state_a.position;
        let dist = delta.length();
        let combined_radius = params_a.radius + params_b.radius;
        
        if dist > combined_radius * 1.2 {
            return None;
        }
        
        // Narrow phase: check if any vertex of A is inside B (using SDF approximation)
        // and vice versa
        let mut max_penetration = 0.0f32;
        let mut best_normal = Vec3::ZERO;
        let mut contact_point = Vec3::ZERO;
        
        // Check A's vertices against B
        for vert_a in &verts_a {
            // SDF of tetrahedron B at vert_a position
            let local_p = params_b.rotation.inverse() * (*vert_a - state_b.position);
            let p_norm = local_p / params_b.radius;
            let sdf_val = ((p_norm.x + p_norm.y).abs() - p_norm.z)
                .max((p_norm.x - p_norm.y).abs() + p_norm.z);
            let signed_dist = (sdf_val - 1.0) / 3.0f32.sqrt() * params_b.radius;
            
            if signed_dist < 0.0 && -signed_dist > max_penetration {
                max_penetration = -signed_dist;
                // Normal points from B towards A (push A away)
                if dist > 1e-6 {
                    best_normal = -delta / dist;
                } else {
                    best_normal = Vec3::Y;
                }
                contact_point = *vert_a;
            }
        }
        
        // Check B's vertices against A
        for vert_b in &verts_b {
            let local_p = params_a.rotation.inverse() * (*vert_b - state_a.position);
            let p_norm = local_p / params_a.radius;
            let sdf_val = ((p_norm.x + p_norm.y).abs() - p_norm.z)
                .max((p_norm.x - p_norm.y).abs() + p_norm.z);
            let signed_dist = (sdf_val - 1.0) / 3.0f32.sqrt() * params_a.radius;
            
            if signed_dist < 0.0 && -signed_dist > max_penetration {
                max_penetration = -signed_dist;
                // Normal points from A towards B (push B away, so normal is positive delta)
                if dist > 1e-6 {
                    best_normal = delta / dist;
                } else {
                    best_normal = Vec3::Y;
                }
                contact_point = *vert_b;
            }
        }
        
        if max_penetration > 0.001 {
            Some(CollisionInfo {
                normal: best_normal,
                penetration_depth: max_penetration,
                contact_point,
            })
        } else {
            None
        }
    }
    
    fn collide_with_floor(
        &self,
        state: &mut PhysicsState,
        floor_y: f32,
        _shape: &dyn Shape,
        shape_params: &ShapeParams,
        dt: f32,
    ) -> bool {
        let verts = Self::get_vertices(state.position, shape_params.radius, state.rotation);
        let (_lowest_idx, lowest_y) = Self::find_lowest_vertex(&verts);
        
        if lowest_y < floor_y {
            // Push up so lowest vertex is at floor
            let penetration = floor_y - lowest_y;
            state.position.y += penetration;
            
            // Bounce only if falling fast
            if state.velocity.y < -0.08 {
                state.velocity.y = -state.velocity.y * 0.2;
            } else {
                state.velocity.y = 0.0;
            }
            
            // Strong friction on floor
            let v_horiz = Vec3::new(state.velocity.x, 0.0, state.velocity.z);
            let horiz_speed = v_horiz.length();
            if horiz_speed > 0.005 {
                let friction = 0.5;
                let friction_decel = friction * 9.81 * dt;
                if horiz_speed < friction_decel {
                    state.velocity.x = 0.0;
                    state.velocity.z = 0.0;
                } else {
                    let friction_dir = -v_horiz.normalize();
                    state.velocity += friction_dir * friction_decel;
                }
            } else {
                state.velocity.x = 0.0;
                state.velocity.z = 0.0;
            }
            
            // Orientation stabilization: gently rotate to rest on a face rather than a vertex
            // Find the face normal closest to pointing down (most stable resting position)
            let face_normals = Self::get_face_normals(state.rotation);
            let mut best_face_idx = 0;
            let mut best_down_dot = face_normals[0].dot(-Vec3::Y);
            for (i, n) in face_normals.iter().enumerate() {
                let down_dot = n.dot(-Vec3::Y);
                if down_dot > best_down_dot {
                    best_down_dot = down_dot;
                    best_face_idx = i;
                }
            }
            
            // If the best face normal isn't already pointing down enough, apply corrective torque
            let target_normal = -Vec3::Y;
            let current_down_face = face_normals[best_face_idx];
            let alignment = current_down_face.dot(target_normal);
            
            if alignment < 0.98 {
                // Calculate rotation axis to align face with floor
                let rotation_axis = current_down_face.cross(target_normal);
                if rotation_axis.length_squared() > 1e-6 {
                    let stabilization_strength = 2.0 * (1.0 - alignment);
                    state.angular_velocity += rotation_axis.normalize() * stabilization_strength * dt;
                }
            }
            
            // Strong angular damping on floor for tetrahedrons
            state.angular_velocity *= 0.8;
            
            // If nearly at rest, zero out angular velocity
            if state.angular_velocity.length_squared() < 0.001 && alignment > 0.95 {
                state.angular_velocity = Vec3::ZERO;
            }
            
            true
        } else {
            false
        }
    }
}

