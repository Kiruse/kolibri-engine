use glam::Vec2;
use wgpu::RenderPass;

use crate::error::EngineError;
use crate::game::{RenderContext, Timings};

/// Scenes are root rendering pipeline entrypoints in Kolibri. They initialize,
/// update & call the rendering pipeline, which includes maintaining, filtering
/// & ordering of the scene graph if applicable.
///
/// Scenes are extremely low-level graphics primitives:
/// - [Scene2D] uses a simple AABB-based filtering with z-level ordering.
/// - [Scene3D] uses a scene graph with various heuristics for filtering, etc.
/// - [ProceduralScene] doesn't support entities at all and instead uses
///   procedural art components.
pub trait Scene {
  /// Initialize the scene, creating render pipeline & such.
  fn init(&mut self, ctx: &RenderContext, timings: &Timings) -> Result<(), EngineError>;

  /// Optional update pass e.g. for updating buffers.
  fn update(
    &mut self,
    #[allow(unused)] queue: &mut wgpu::Queue,
    #[allow(unused)] timings: &Timings,
  ) -> Result<(), EngineError> {
    Ok(())
  }

  fn resize(
    &mut self,
    #[allow(unused)] queue: &mut wgpu::Queue,
    #[allow(unused)] new_size: Vec2,
  ) -> Result<(), EngineError> {
    Ok(())
  }

  /// Render this scene using the given render pass.
  fn render(&mut self, pass: &mut RenderPass) -> Result<(), EngineError>;
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimingsUniform {
  pub delta_time: f32,
  pub world_time: f32,
  pub scene_time: f32,
}

impl TimingsUniform {
  pub fn new(world_time: f32) -> Self {
    Self {
      delta_time: 0.,
      world_time,
      scene_time: 0.,
    }
  }
}
