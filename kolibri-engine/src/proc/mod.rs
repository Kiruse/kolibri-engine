use std::borrow::Cow;
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{Arc, mpsc};

use egui::TextBuffer;
use glam::{Vec2, Vec3};
use notify::{Event as NotifyEvent, EventKind as NotifyEventKind, RecommendedWatcher, Result as NotifyResult, Watcher};
use wgpu::{Device, RenderPass, util::DeviceExt};

use crate::error::EngineError;
use crate::game::{RenderContext, Timings};
use super::scene::Scene;

pub trait FragmentFactory {
  /// Load the fragment shader module
  fn load(&self, device: &Device) -> Result<wgpu::ShaderModule, anyhow::Error>;

  /// Whether this fragment factory wants to reload its fragment shader module
  #[inline]
  fn take_dirty(&mut self) -> bool { false }
}

pub struct FixedFragmentFactory {
  loader: fn(&Device) -> Result<wgpu::ShaderModule, anyhow::Error>,
}

impl FragmentFactory for FixedFragmentFactory {
  #[inline]
  fn load(&self, device: &Device) -> Result<wgpu::ShaderModule, anyhow::Error> {
    (self.loader)(device)
  }
}

impl FixedFragmentFactory {
  pub fn new(loader: fn(&Device) -> Result<wgpu::ShaderModule, anyhow::Error>) -> Self {
    Self { loader }
  }
}

// #[cfg(hmr)]
pub struct HMRFragmentFactory {
  path: PathBuf,
  // Watcher is stored here so it lives as long as the factory itself
  #[allow(unused)]
  watcher: RecommendedWatcher,
  dirty: Arc<AtomicBool>,
}

impl FragmentFactory for HMRFragmentFactory {
  fn load(&self, device: &Device) -> Result<wgpu::ShaderModule, anyhow::Error> {
    let src = std::fs::read_to_string(self.path.to_string_lossy().as_str())?;
    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some(&self.path.to_string_lossy()),
      source: wgpu::ShaderSource::Wgsl(Cow::Owned(src))
    }))
  }

  #[inline]
  fn take_dirty(&mut self) -> bool {
    self.dirty.swap(false, AtomicOrdering::Relaxed)
  }
}

impl HMRFragmentFactory {
  pub fn new(path: PathBuf) -> Self {
    let (tx, rx) = mpsc::channel::<NotifyResult<NotifyEvent>>();

    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default()).expect("Failed to initialize filesystem watcher");
    watcher.watch(&path, notify::RecursiveMode::NonRecursive).expect("Failed to watch filesystem");

    let dirty: Arc<AtomicBool> = Default::default();
    let inner_dirty = dirty.clone();
    let inner_path = path.clone();

    std::thread::spawn(move || {
      for event in rx {
        if let Ok(event) = event && matches!(event.kind, NotifyEventKind::Modify(_)) {
          println!("update {}", inner_path.to_string_lossy().as_str());
          inner_dirty.store(true, AtomicOrdering::Relaxed);
        }
      }
    });

    Self {
      path,
      watcher,
      dirty,
    }
  }
}

#[cfg(not(feature = "hmr"))]
#[macro_export]
macro_rules! frag {
  ($s:literal) => {
    kolibri_engine::proc::FixedFragmentFactory::new(|device| Ok(device.create_shader_module(wgpu::include_wgsl!($s))))
  }
}

#[cfg(feature = "hmr")]
#[macro_export]
macro_rules! frag {
  ($s:literal) => {
    kolibri_engine::proc::HMRFragmentFactory::new(std::path::Path::new(file!()).parent().unwrap().join($s))
  }
}

/// Procedural Scene operates on the assumption that you will provide
/// the fragment shader operating on a neutral plane, using procedures
/// to compute the fragment color at this pixel.
///
/// The engine provides various uniforms in bind group 0. The fragment
/// shader's entrypoint function name is *expected* to be `fx_main`.
///
/// ```wgsl
/// // Pseudo-static data that very rarely changes
/// @group(0) @binding(0)
/// var<uniform> statics: Static;
///
/// struct Static {
///   // width/height of the scene
///   size: vec2f,
/// }
///
/// // Timing data that changes every frame
/// @group(0) @binding(1)
/// var<uniform> timing: Timings;
///
/// struct Timings {
///   // Time since last frame
///   delta_time: f32,
///   // Total app lifetime
///   world_time: f32,
///   // Total scene lifetime
///   scene_time: f32,
/// }
///
/// // Camera data
/// @group(0) @binding(2)
/// var<uniform> camera: Camera;
///
/// struct Camera {
///   // Field of view (along Y axis)
///   fov: f32,
///   // Aspect ratio (width/height)
///   aspect: f32,
///   // Camera forward vector (default Z axis unit vector)
///   forward: vec3f,
///   // Camera right vector (default X axis unit vector)
///   right: vec3f,
///   // Camera up vector (default Y axis unit vector)
///   up: vec3f,
/// }
///
/// @fragment
/// fn fx_main(@builtin(position) pos: vec4f) -> @location(0) vec4f {
///   // ...
/// }
/// ```
pub struct ProceduralScene<F: FragmentFactory> {
  pub scene_size: Vec2,
  state: Option<ProceduralSceneState>,
  fragment_factory: F,
}

pub struct ProceduralSceneState {
  pipeline: wgpu::RenderPipeline,
  bindgroup: wgpu::BindGroup,
  buf_static: wgpu::Buffer,
  buf_timings: wgpu::Buffer,
  #[allow(unused)]
  buf_camera: wgpu::Buffer,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct StaticUniform {
  scene_size: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
  fov: f32,          // size  4
  aspect: f32,       // size  4, offset 4
  _pad0: [f32; 2],   // size  8, offset 8
  forward: [f32; 3], // size 12, offset 16
  _pad1: f32,        // size  4, offset 28
  right: [f32; 3],   // size 12, offset 32
  _pad2: f32,        // size  4, offset 44
  up: [f32; 3],      // size 12, offset 48
  _pad3: f32,        // size  4, offset 60
  // total 64
}

impl CameraUniform {
  fn new(fov: f32, aspect: f32, forward: Vec3, right: Vec3, up: Vec3) -> Self {
    Self {
      fov,
      aspect,
      forward: forward.to_array(),
      right: right.to_array(),
      up: up.to_array(),
      ..Default::default()
    }
  }

  fn vfov(hfov: f32, aspect: f32) -> f32 {
    2. * ((hfov / 2.).tan() / aspect).atan()
  }

  #[allow(unused)]
  fn hfov(vfov: f32, aspect: f32) -> f32 {
    2. * ((vfov / 2.).tan() * aspect).atan()
  }
}

impl<F: FragmentFactory> Scene for ProceduralScene<F> {
  fn init(&mut self, ctx: &RenderContext, timings: &Timings) -> Result<(), EngineError> {
    let vx_shader = ctx.device.create_shader_module(wgpu::include_wgsl!("proc.wgsl"));
    let fx_shader = self.fragment_factory.load(&ctx.device)?;

    let uf_static = StaticUniform {
      scene_size: self.scene_size.to_array(),
    };

    let buf_static = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("ProceduralScene static uniform buffer"),
      contents: bytemuck::cast_slice(&[uf_static]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let buf_timings = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("ProceduralScene timings uniform buffer"),
      contents: bytemuck::cast_slice(&[timings.as_uniform()]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // TODO: this should probably be based on scene size...
    let aspect = 16./9.;
    let uf_camera = CameraUniform::new(
      CameraUniform::vfov(90., aspect),
      aspect,
      Vec3::new(0., 0., 1.),
      Vec3::new(1., 0., 0.),
      Vec3::new(0., 1., 0.),
    );

    let buf_camera = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("ProceduralScene camera uniform buffer"),
      contents: bytemuck::cast_slice(&[uf_camera]),
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
        module: &vx_shader,
        entry_point: Some("vx_main"),
        buffers: &[],
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &fx_shader,
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

    self.state = Some(ProceduralSceneState {
      pipeline,
      bindgroup,
      buf_static,
      buf_timings,
      buf_camera,
    });

    Ok(())
  }

  fn update(&mut self, queue: &mut wgpu::Queue, timings: &Timings) -> Result<(), EngineError> {
    let state = self.state()?;
    queue.write_buffer(&state.buf_timings, 0, bytemuck::cast_slice(&[timings.as_uniform()]));
    // TODO: navigable perspective camera?
    Ok(())
  }

  fn resize(
    &mut self,
    queue: &mut wgpu::Queue,
    new_size: Vec2,
  ) -> Result<(), EngineError> {
    self.scene_size = new_size;

    let state = self.state()?;
    let uf = StaticUniform {
      scene_size: new_size.to_array(),
    };
    queue.write_buffer(&state.buf_static, 0, bytemuck::cast_slice(&[uf]));
    Ok(())
  }

  fn render(&mut self, pass: &mut RenderPass) -> Result<(), EngineError> {
    let state = self.state()?;
    pass.set_pipeline(&state.pipeline);
    pass.set_bind_group(0, &state.bindgroup, &[]);
    pass.draw(0..4, 0..1);
    Ok(())
  }
}

impl<F: FragmentFactory> ProceduralScene<F> {
  pub fn new(fragment_factory: F) -> Self {
    Self {
      scene_size: Vec2::default(),
      state: None,
      fragment_factory,
    }
  }

  fn state(&self) -> Result<&ProceduralSceneState, EngineError> {
    self.state.as_ref().ok_or(EngineError::state("ProceduralScene render pipeline not initialized"))
  }
}
