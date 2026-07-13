use std::collections::HashMap;
use std::num::NonZero;

use encase::ShaderType;
use glam::Vec2;
use wgpu::util::DeviceExt;

use crate::camera::{FieldOfView, PerspectiveCamera};
use crate::error::EngineError;
use crate::game::{RenderContext, Timings};
use crate::octree::{ChunkCoord, Octree};
use crate::scene::Scene;
use crate::util::to_buffer;

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

#[derive(Debug, Clone)]
struct State {
  pipeline: wgpu::RenderPipeline,
  bindgroup: wgpu::BindGroup,
  buf_static: wgpu::Buffer,
  buf_timings: wgpu::Buffer,
  buf_camera: wgpu::Buffer,
}

#[derive(ShaderType)]
struct StaticUniform {
  scene_size: Vec2,
}

pub struct VoxelScene {
  scene_size: Vec2,
  camera: PerspectiveCamera,
  terrain: HashMap<[i32; 3], OctreeRoot>,
  state: Option<State>,
}

impl VoxelScene {
  fn state(&self) -> Result<&State, EngineError> {
    self.state.as_ref().ok_or(EngineError::state("VoxelScene render pipeline not initialized"))
  }

  pub fn set_fov(&mut self, fov: FieldOfView) {
    self.camera.fov = fov;
  }

  pub fn fov(&self) -> FieldOfView {
    self.camera.fov
  }
}

impl Scene for VoxelScene {
  fn init(&mut self, ctx: &RenderContext, timings: &Timings, scene_size: Vec2) -> Result<(), crate::error::EngineError> {
    self.scene_size = scene_size;
    self.camera.aspect = scene_size.x / scene_size.y;

    let shader = ctx.device.create_shader_module(wgpu::include_wgsl!("voxel.wgsl"));

    let uf_static = StaticUniform {
      scene_size: self.scene_size,
    };

    let buf_static = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("ProceduralScene static uniform buffer"),
      contents: &to_buffer(&uf_static),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let buf_timings = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("ProceduralScene timings uniform buffer"),
      contents: &to_buffer(&timings.as_uniform()),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let buf_camera = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("ProceduralScene camera uniform buffer"),
      contents: &self.camera.to_buffer(),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bgl = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("ProceduralScene bind group layout"),
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZero::new(4u64 * 2u64),
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZero::new(4u64 * 3u64),
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZero::new(64u64),
          },
          count: None,
        },
      ],
    });

    let bindgroup = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("ProceduralScene bind group camera"),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: buf_static.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: buf_timings.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: buf_camera.as_entire_binding(),
        },
      ],
      layout: &bgl,
    });

    let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Procedural Pipeline Layout"),
      bind_group_layouts: &[Some(&bgl)],
      ..Default::default()
    });

    let pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Procedural Pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vx_main"),
        buffers: &[],
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fx_main"),
        compilation_options: Default::default(),
        targets: &[Some(wgpu::ColorTargetState {
          format: ctx.surface_cfg.format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::ALL,
        })],
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        cull_mode: None,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      multiview_mask: None,
      cache: None,
    });

    self.state = Some(State {
      pipeline,
      bindgroup,
      buf_static,
      buf_timings,
      buf_camera,
    });

    Ok(())
  }

  fn render(&mut self, pass: &mut wgpu::RenderPass) -> Result<(), crate::error::EngineError> {
    let state = self.state()?;
    pass.set_pipeline(&state.pipeline);
    pass.set_bind_group(0, &state.bindgroup, &[]);
    pass.draw(0..4, 0..1);
    Ok(())
  }

  fn resize(
    &mut self,
    queue: &mut wgpu::Queue,
    new_size: glam::Vec2,
  ) -> Result<(), crate::error::EngineError> {
    self.scene_size = new_size;
    self.camera.aspect = new_size.x / new_size.y;

    let state = self.state()?;
    let uf = StaticUniform {
      scene_size: new_size,
    };

    queue.write_buffer(&state.buf_static, 0, &to_buffer(&uf));
    Ok(())
  }

  fn update(
    &mut self,
    queue: &mut wgpu::Queue,
    timings: &crate::prelude::Timings,
  ) -> Result<(), crate::error::EngineError> {
    let state = self.state()?;
    // TODO: navigate w/ keyboard and/or gamepad inputs
    // Camera generally updates so frequently we don't really care to distinguish if it's dirty
    queue.write_buffer(&state.buf_timings, 0, &to_buffer(&timings.as_uniform()));
    queue.write_buffer(&state.buf_camera, 0, &self.camera.to_buffer());
    Ok(())
  }
}

impl Default for VoxelScene {
  fn default() -> Self {
    Self {
      scene_size: Vec2::ONE,
      camera: PerspectiveCamera::default(),
      terrain: Default::default(),
      state: None,
    }
  }
}
