use std::time::Instant;

use glam::Vec2;
use log::{debug, error};
use wgpu::CurrentSurfaceTexture;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::error::EngineError;
use crate::scene::{Scene, TimingsUniform};

pub struct Game {
  ctx: Option<RenderContext>,
  scene: Option<Box<dyn Scene>>,
  looper: Option<Box<dyn FnMut() -> Result<Option<Box<dyn Scene>>, anyhow::Error>>>,
  timings: Timings,
}

pub struct RenderContext {
  pub size: PhysicalSize<u32>,
  pub surface: wgpu::Surface<'static>,
  pub surface_cfg: wgpu::SurfaceConfiguration,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
}

impl Game {
  pub fn new() -> Self {
    Self {
      ctx: None,
      scene: None,
      looper: None,
      timings: Timings::new(),
    }
  }

  async fn init(&mut self, window: Window) -> Result<(), EngineError> {
    let instance = wgpu::Instance::default();
    let size = window.inner_size();
    let surface = instance.create_surface(window)?;
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
      })
      .await?;

    let (device, queue) = adapter
      .request_device(&wgpu::DeviceDescriptor {
        ..Default::default()
      })
      .await?;

    let config = surface
      .get_default_config(&adapter, size.width.max(1), size.height.max(1))
      .ok_or(EngineError::init("failed to configure render surface"))?;
    surface.configure(&device, &config);

    self.ctx = Some(RenderContext {
      size,
      surface,
      surface_cfg: config,
      device,
      queue,
    });

    Ok(())
  }

  fn update(&mut self) -> Result<(), EngineError> {
    let Some(ctx) = self.ctx.as_mut() else {
      return Ok(());
    };

    if let Some(looper) = self.looper.as_mut() {
      if let Some(mut next_scene) = looper()? {
        self.timings.reset_scene();
        next_scene.init(ctx, &self.timings)?;
        next_scene.resize(&mut ctx.queue, Vec2::new(ctx.size.width as f32, ctx.size.height as f32))?;
        self.scene = Some(next_scene);
      }
    }

    if let Some(scene) = self.scene.as_mut() {
      scene.update(&mut ctx.queue, &self.timings)?;
    }

    self.timings.tick();
    Ok(())
  }

  fn render(&mut self) -> Result<(), EngineError> {
    let Some(RenderContext {
      surface,
      surface_cfg,
      device,
      queue,
      ..
    }) = &self.ctx else {
      return Ok(());
    };
    let Some(scene) = self.scene.as_mut() else {
      debug!("No scene to render");
      return Ok(());
    };

    let output = match surface.get_current_texture() {
      CurrentSurfaceTexture::Success(frame) => frame,
      CurrentSurfaceTexture::Suboptimal(frame) => {
        drop(frame);
        surface.configure(device, surface_cfg);
        match surface.get_current_texture() {
          CurrentSurfaceTexture::Success(frame) => frame,
          _ => return Ok(()), // give up this frame, try again next tick
        }
      },
      CurrentSurfaceTexture::Outdated => {
        surface.configure(device, surface_cfg);
        return Ok(());
      }
      x => {
        eprintln!("Unhandled texture: {x:?}");
        self.ctx = None;
        return Ok(());
      },
    };

    let view = output
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("GameWindow::render encoder") });

    scene.render(&mut encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("GameWindow::render pass"),
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        depth_slice: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color {
            r: 0.1,
            g: 0.1,
            b: 0.15,
            a: 1.0,
          }),
          store: wgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
      multiview_mask: None,
    }))?;

    queue.submit([encoder.finish()]);
    output.present();
    Ok(())
  }

  /// Convenience helper for creating a standard [winit::event_loop::EventLoop].
  pub fn make_event_loop() -> Result<EventLoop<()>, EngineError> {
    let evloop = EventLoop::new()?;
    evloop.set_control_flow(ControlFlow::Poll);
    Ok(evloop)
  }

  pub fn run(
    looper: impl FnMut() -> Result<Option<Box<dyn Scene>>, anyhow::Error> + 'static
  ) -> Result<(), EngineError> {
    let mut game = Self::new();
    game.looper = Some(Box::new(looper));
    let event_loop = Self::make_event_loop()?;
    event_loop.run_app(&mut game)?;
    Ok(())
  }
}

impl ApplicationHandler for Game {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window = event_loop
      .create_window(
        Window::default_attributes()
          .with_title("Kolibri Engine")
          .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
      )
      .expect("Failed to create window");

    if let Err(e) = pollster::block_on(self.init(window)) {
      eprintln!("GPU init error: {e}");
    }
  }

  fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    self.ctx = None;
  }

  fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    if let Err(e) = self.update() {
      eprintln!("Error during update: {e}");
      return;
    }
    if let Err(e) = self.render() {
      eprintln!("Error during render: {e}");
      return;
    }
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: WindowId,
    event: WindowEvent,
  ) {
    match event {
      WindowEvent::CloseRequested => event_loop.exit(),
      WindowEvent::Resized(new_size) => {
        let Some(RenderContext { size, surface, surface_cfg, device, queue }) = self.ctx.as_mut() else { return };

        surface_cfg.width = new_size.width.max(1);
        surface_cfg.height = new_size.height.max(1);
        surface.configure(device, &surface_cfg);
        *size = new_size;

        if let Some(scene) = self.scene.as_mut() {
          if let Err(e) = scene.resize(queue, Vec2::new(new_size.width as f32, new_size.height as f32)) {
            error!("Scene failed to handle resize: {e}");
          }
        }
      }
      // TODO: handle inputs
      _ => {}
    }
  }
}

pub struct Timings {
  pub world_start: Instant,
  pub scene_start: Instant,
  pub last_frame: Instant,
}

impl Timings {
  fn new() -> Self {
    let now = Instant::now();
    Self {
      world_start: now.clone(),
      scene_start: now.clone(),
      last_frame: now,
    }
  }

  fn reset_scene(&mut self) {
    self.scene_start = Instant::now();
  }

  fn tick(&mut self) {
    self.last_frame = Instant::now();
  }

  pub fn as_uniform(&self) -> TimingsUniform {
    TimingsUniform {
      delta_time: self.last_frame.elapsed().as_secs_f32(),
      scene_time: self.scene_start.elapsed().as_secs_f32(),
      world_time: self.world_start.elapsed().as_secs_f32(),
    }
  }
}
