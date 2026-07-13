use encase::ShaderType;
use glam::{Mat3, Vec3};

use crate::transform::Transform;

/// Generalized Perspective Camera
pub struct PerspectiveCamera {
  pub fov: FieldOfView,
  pub aspect: f32,
  pub transform: Transform,
}

impl PerspectiveCamera {
  pub fn new(fov: FieldOfView, aspect: f32) -> Self {
    Self {
      fov,
      aspect,
      transform: Transform::default(),
    }
  }

  /// Serialize this camera to its uniform buffer contents. The uniform is expected to
  /// match this WGSL struct:
  ///
  /// ```wgsl
  /// struct Camera {
  ///   rotation: mat3x3f,
  ///   location: vec3f,
  ///   tan_half_fov: f32, // tan(fov/2)
  ///   aspect: f32,
  /// }
  /// ```
  pub fn to_buffer(&self) -> Vec<u8> {
    crate::util::to_buffer(&PerspectiveCameraUniform {
      rotation: Mat3::from_mat4(self.transform.rotation_mat()),
      location: self.transform.location().clone(),
      tan_half_fov: (self.fov.vertical(self.aspect)/2.).tan(),
      aspect: self.aspect,
    })
  }

  /// Compute the vertical field of view from the given horizontal one + aspect ratio.
  pub fn vfov(hfov: f32, aspect: f32) -> f32 {
    2. * ((hfov / 2.).tan() / aspect).atan()
  }

  /// Compute the horizontal field of view from the given vertical one + aspect ratio.
  pub fn hfov(vfov: f32, aspect: f32) -> f32 {
    2. * ((vfov / 2.).tan() * aspect).atan()
  }
}

impl Default for PerspectiveCamera {
  fn default() -> Self {
    Self::new(FieldOfView::Horizontal(90.), 1.)
  }
}

#[derive(ShaderType)]
struct PerspectiveCameraUniform {
  rotation: Mat3,
  location: Vec3,
  tan_half_fov: f32,
  aspect: f32,
}

#[derive(Debug, Copy, Clone)]
pub enum FieldOfView {
  Vertical(f32),
  Horizontal(f32),
}

impl FieldOfView {
  /// Convert this field of view to a vertical field of view. Used internally for rendering.
  pub fn vertical(&self, aspect: f32) -> f32 {
    match self {
      Self::Vertical(fov) => *fov,
      Self::Horizontal(hfov) => PerspectiveCamera::vfov(*hfov, aspect),
    }
  }
}
