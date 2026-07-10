use std::collections::HashMap;

use crate::{octree::{ChunkCoord, Octree}, scene::Scene};

#[derive(Clone, Default)]
struct OctreeRoot {
  root: Octree,
  dirty: bool,
}

impl OctreeRoot {
  pub fn set_voxel(&mut self, position: ChunkCoord, id: u32) {
    todo!();
  }

  pub fn clear_voxel(&mut self, position: ChunkCoord) {
    self.set_voxel(position, 0)
  }
}

pub struct VoxelScene {
  terrain: HashMap<[i32; 3], OctreeRoot>,
}

impl Scene for VoxelScene {
  fn init(&mut self, ctx: &crate::game::RenderContext, timings: &crate::prelude::Timings) -> Result<(), crate::error::EngineError> {
    todo!()
  }

  fn render(&mut self, pass: &mut wgpu::RenderPass) -> Result<(), crate::error::EngineError> {
    todo!()
  }

  fn resize(
    &mut self,
    _queue: &mut wgpu::Queue,
    _new_size: glam::Vec2,
  ) -> Result<(), crate::error::EngineError> {
    todo!()
  }

  fn update(
    &mut self,
    _queue: &mut wgpu::Queue,
    _timings: &crate::prelude::Timings,
  ) -> Result<(), crate::error::EngineError> {
    todo!()
  }
}

impl VoxelScene {
  pub fn new() -> Self {
    Self {
      terrain: Default::default(),
    }
  }
}
