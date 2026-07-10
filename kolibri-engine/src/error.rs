use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
  #[error("Failed to create event loop: {0}")]
  EventLoop(#[from] winit::error::EventLoopError),
  #[error("Failed to create window: {0}")]
  WindowCreation(#[from] winit::error::OsError),
  #[error("WGPU error: {0}")]
  Wgpu(#[from] wgpu::Error),
  #[error("Failed to create surface: {0}")]
  CreateSurface(#[from] wgpu::CreateSurfaceError),
  #[error("Failed to request adapter: {0}")]
  RequestAdapter(#[from] wgpu::RequestAdapterError),
  #[error("Failed to request device: {0}")]
  RequestDevice(#[from] wgpu::RequestDeviceError),
  #[error("{0}")]
  Custom(#[from] anyhow::Error),
  #[error("No suitable GPU adapter found")]
  NoAdapter,
  #[error("Error during initialization: {0}")]
  Init(String),
  #[error("State error: {0}")]
  State(String),
  #[error("Input error: {0}")]
  Input(String),
}

impl EngineError {
  pub fn init(msg: impl Into<String>) -> Self {
    Self::Init(msg.into())
  }

  pub fn state(msg: impl Into<String>) -> Self {
    Self::State(msg.into())
  }

  pub fn input(msg: impl Into<String>) -> Self {
    Self::State(msg.into())
  }
}
