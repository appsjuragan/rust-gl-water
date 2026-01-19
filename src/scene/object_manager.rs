//! Object manager for scene lifecycle management

use super::scene_object::{SceneObject, ObjectId};
use std::collections::HashMap;

/// Manages all scene objects
#[allow(dead_code)]
pub struct ObjectManager {
    objects: HashMap<ObjectId, SceneObject>,
    next_id: usize,
}

impl ObjectManager {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            next_id: 0,
        }
    }

    #[allow(dead_code)]
    pub fn add_object(&mut self, mut object: SceneObject) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        object.id = id;
        self.objects.insert(id, object);
        id
    }

    #[allow(dead_code)]
    pub fn remove_object(&mut self, id: ObjectId) -> Option<SceneObject> {
        self.objects.remove(&id)
    }

    #[allow(dead_code)]
    pub fn get(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.get(&id)
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut SceneObject> {
        self.objects.get_mut(&id)
    }

    #[allow(dead_code)]
    pub fn objects(&self) -> impl Iterator<Item = &SceneObject> {
        self.objects.values()
    }

    #[allow(dead_code)]
    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut SceneObject> {
        self.objects.values_mut()
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.objects.len()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.objects.clear();
    }

    #[allow(dead_code)]
    pub fn update_physics(&mut self, dt: f32, gravity: f32, water_height: f32, pool_type: crate::core::physics_trait::PoolType) {
        let water_density = 1000.0;

        for object in self.objects.values_mut() {
            object.integrate_physics(dt, gravity, water_height, water_density);
            object.collide_with_pool(pool_type);
        }

        self.solve_collisions();
    }

    fn solve_collisions(&mut self) {
        let mut collisions = Vec::new();
        let ids: Vec<ObjectId> = self.objects.keys().copied().collect();

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let id_a = ids[i];
                let id_b = ids[j];

                let obj_a = &self.objects[&id_a];
                let obj_b = &self.objects[&id_b];
                
                if let Some(info) = obj_a.check_collision(obj_b) {
                    collisions.push((id_a, id_b, info));
                }
            }
        }

        for (id_a, id_b, info) in collisions {
            if let Some(obj_a) = self.objects.get_mut(&id_a) {
                obj_a.physics.position -= info.normal * info.penetration_depth * 0.5;
            }
            if let Some(obj_b) = self.objects.get_mut(&id_b) {
                obj_b.physics.position += info.normal * info.penetration_depth * 0.5;
            }

            let vel_a = self.objects[&id_a].physics.velocity;
            let vel_b = self.objects[&id_b].physics.velocity;
            
            let relative_vel = vel_b - vel_a;
            let vel_along_normal = relative_vel.dot(info.normal);

            if vel_along_normal < 0.0 {
                let impulse = info.normal * vel_along_normal * 0.5;
                
                if let Some(obj_a) = self.objects.get_mut(&id_a) {
                    obj_a.physics.velocity += impulse;
                }
                if let Some(obj_b) = self.objects.get_mut(&id_b) {
                    obj_b.physics.velocity -= impulse;
                }
            }
        }
    }
}

impl Default for ObjectManager {
    fn default() -> Self {
        Self::new()
    }
}
