//! Scene management module

pub mod scene_object;
pub mod object_manager;
pub mod collider_impl;

#[allow(unused_imports)]
pub use scene_object::{SceneObject, ObjectId, Transform};
#[allow(unused_imports)]
pub use object_manager::ObjectManager;
#[allow(unused_imports)]
pub use collider_impl::{BoundingSphereCollider, StandardBuoyancy};
