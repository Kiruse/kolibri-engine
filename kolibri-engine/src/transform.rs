use std::cell::RefCell;

use glam::{Mat3, Mat4, Quat, Vec3, Vec4};

/// "Grounded" transform, i.e. an entity that moves along a surface,
/// typically along world gravity, but generalized to any gravity direction.
#[derive(Debug, Clone)]
pub struct Transform {
  location: Vec3,
  gravity: Vec3,
  right: Vec3,
  scale: Vec3,
  yaw: f32,
  pitch: f32,
  cache: RefCell<TransformCache>,
}

impl Transform {
  pub fn new(location: Vec3) -> Self {
    Self {
      location,
      gravity: Vec3::new(0., -9.81, 0.),
      right: Vec3::new(1., 0., 0.),
      scale: Vec3::ONE,
      yaw: 0.,
      pitch: 0.,
      cache: RefCell::new(TransformCache::default())
    }
  }

  /// Create a new transform with the given location.
  pub fn with_location(self, location: Vec3) -> Self {
    Self {
      location,
      cache: RefCell::new(TransformCache::default()),
      ..self
    }
  }

  /// Create a new transform with the given scalar applied as scale across all 3 axes.
  pub fn with_scalar(self, s: f32) -> Self {
    Self {
      scale: Vec3::new(s, s, s),
      cache: RefCell::new(TransformCache::default()),
      ..self
    }
  }

  /// Create a new transform with the given scale.
  pub fn with_scale(self, scale: Vec3) -> Self {
    Self {
      scale,
      cache: RefCell::new(TransformCache::default()),
      ..self
    }
  }

  /// Create a new transform with the given gravity acceleration vector.
  pub fn with_gravity(self, gravity: Vec3) -> Self {
    // edge case: `Quat::from_rotation_arc` computes the *shortest angle*
    // between 2 vectors, but there is no shortest vector when they're
    // anti-parallel. in that case, simply flip the right vector.
    let right = if gravity.dot(self.gravity) < -0.9999 {
      -self.right
    } else {
      let g = gravity.normalize();
      let rot = Quat::from_rotation_arc(self.gravity.normalize(), g);
      let r = rot * self.right;
      (r - g * r.dot(g)).normalize()
    };

    Self {
      gravity,
      right,
      cache: RefCell::new(TransformCache::default()),
      ..self
    }
  }

  /// Create a new transform with the given local yaw around the gravity vector.
  pub fn with_yaw(self, yaw: f32) -> Self {
    Self {
      yaw,
      cache: RefCell::new(TransformCache::default()),
      ..self
    }
  }

  /// Create a new transform with the given local pitch around a stable-ish axis perpendicular to the gravity vector.
  pub fn with_pitch(self, pitch: f32) -> Self {
    Self {
      pitch,
      cache: RefCell::new(TransformCache::default()),
      ..self
    }
  }

  pub fn location(&self) -> &Vec3 {
    &self.location
  }

  /// Stored scale vector
  pub fn scale(&self) -> &Vec3 {
    &self.scale
  }

  /// Stored elemental yaw around the [Transform::gravity] vector.
  pub fn yaw(&self) -> f32 {
    self.yaw
  }

  /// Stored elemental pitch around the [Transform::right] vector.
  pub fn pitch(&self) -> f32 {
    self.pitch
  }

  /// Stored gravity acceleration vector.
  pub fn gravity(&self) -> &Vec3 {
    &self.gravity
  }

  /// Compute the up vector of this transform from its `gravity`.
  pub fn up(&self) -> Vec3 {
    -self.gravity.normalize()
  }

  /// Right vector of this transform, perpendicular to [gravity]. This is automatically
  /// updated whenever gravity is changed with [Transform::with_gravity].
  pub fn right(&self) -> &Vec3 {
    &self.right
  }

  /// Compute the forward vector of this transform from its `gravity` and `right` vectors.
  pub fn forward(&self) -> Vec3 {
    self.right.cross(self.up())
  }

  /// Get local rotation quaternion based on transform's local gravity & right vectors.
  pub fn local_quat(&self) -> Quat {
    let q_yaw   = Quat::from_axis_angle(self.up(), self.yaw);
    let q_pitch = Quat::from_axis_angle(self.right, self.pitch);
    q_yaw * q_pitch
  }

  /// Get global rotation quaternion.
  pub fn global_quat(&self) -> Quat {
    let up = -self.gravity.normalize();
    let forward = self.right.cross(up);
    let q_base = Quat::from_mat3(&Mat3::from_cols(self.right, up, forward));
    self.local_quat() * q_base
  }

  pub fn scale_mat(&self) -> Mat4 {
    Mat4::from_cols(
      Vec4::new(self.scale.x, 0., 0., 0.),
      Vec4::new(0., self.scale.y, 0., 0.),
      Vec4::new(0., 0., self.scale.z, 0.),
      Vec4::new(0., 0., 0., 1.),
    )
  }

  pub fn inv_scale_mat(&self) -> Mat4 {
    Mat4::from_cols(
      Vec4::new(1./self.scale.x, 0., 0., 0.),
      Vec4::new(0., 1./self.scale.y, 0., 0.),
      Vec4::new(0., 0., 1./self.scale.z, 0.),
      Vec4::new(0., 0., 0., 1.),
    )
  }

  pub fn rotation_mat(&self) -> Mat4 {
    let q = self.global_quat();
    Mat4::from_cols(
      Vec4::new(1. - 2.*(q.y*q.y + q.z*q.z), 2.*(q.x*q.y + q.w*q.z), 2.*(q.x*q.z - q.w*q.y), 0.),
      Vec4::new(2.*(q.x*q.y - q.w*q.z), 1. - 2.*(q.x*q.x + q.z*q.z), 2.*(q.y*q.z - q.w*q.x), 0.),
      Vec4::new(2.*(q.x*q.z + q.w*q.y), 2.*(q.y*q.z - q.w*q.x), 1. - 2.*(q.x*q.x + q.y*q.y), 0.),
      Vec4::new(0., 0., 0., 1.),
    )
  }

  pub fn inv_rotation_mat(&self) -> Mat4 {
    let q = self.global_quat().inverse();
    Mat4::from_cols(
      Vec4::new(1. - 2.*(q.y*q.y + q.z*q.z), 2.*(q.x*q.y + q.w*q.z), 2.*(q.x*q.z - q.w*q.y), 0.),
      Vec4::new(2.*(q.x*q.y - q.w*q.z), 1. - 2.*(q.x*q.x + q.z*q.z), 2.*(q.y*q.z - q.w*q.x), 0.),
      Vec4::new(2.*(q.x*q.z + q.w*q.y), 2.*(q.y*q.z - q.w*q.x), 1. - 2.*(q.x*q.x + q.y*q.y), 0.),
      Vec4::new(0., 0., 0., 1.),
    )
  }

  pub fn translation_mat(&self) -> Mat4 {
    let l = &self.location;
    Mat4::from_cols(
      Vec4::new(1., 0., 0., 0.),
      Vec4::new(0., 1., 0., 0.),
      Vec4::new(0., 0., 1., 0.),
      Vec4::new(l.x, l.y, l.z, 1.),
    )
  }

  pub fn inv_translation_mat(&self) -> Mat4 {
    let l = &self.location;
    Mat4::from_cols(
      Vec4::new(1., 0., 0., 0.),
      Vec4::new(0., 1., 0., 0.),
      Vec4::new(0., 0., 1., 0.),
      Vec4::new(-l.x, -l.y, -l.z, 1.),
    )
  }

  fn update_cache(&self) {
    let mut cache = self.cache.borrow_mut();
    if cache.dirty {
      cache.dirty = false;
      cache.mat = self.translation_mat() * self.rotation_mat() * self.scale_mat();
      cache.inv_mat = self.inv_scale_mat() * self.inv_rotation_mat() * self.inv_translation_mat()
    }
  }

  /// Calculate the local-to-global transformation matrix of this transform.
  pub fn to_matrix(&self) -> Mat4 {
    self.update_cache();
    self.cache.borrow().mat
  }

  /// Calculate the global-to-local inverse transformation matrix of this transform.
  /// This matrix is used within the engine for raymarching during rendering.
  pub fn to_inv_matrix(&self) -> Mat4 {
    self.update_cache();
    self.cache.borrow().inv_mat
  }
}

impl Default for Transform {
  fn default() -> Self {
    Self::new(Vec3::ZERO)
  }
}

#[derive(Debug, Clone)]
struct TransformCache {
  dirty: bool,
  mat: Mat4,
  inv_mat: Mat4,
}

impl Default for TransformCache {
  fn default() -> Self {
    Self {
      dirty: true,
      mat: Mat4::IDENTITY,
      inv_mat: Mat4::IDENTITY,
    }
  }
}
