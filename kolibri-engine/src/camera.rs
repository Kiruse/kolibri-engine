use glam::Mat4;

pub trait Camera {
  /// Get the projection matrix to convert world vertices into clip space vertices.
  fn projection_matrix(&self) -> Mat4;
}
