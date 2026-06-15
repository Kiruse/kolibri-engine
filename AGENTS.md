# kolibri-engine

2D/3D game engine built on wgpu + winit + egui + glam. Edition 2024. Cargo workspace with two crates:
- `kolibri-engine/` — main engine library (no binary; run examples with `cargo run -p kolibri-engine --example <name>`).
- `kolibri-derive/` — proc macro crate providing `#[kolibri_derive::define(crate::PRF)]`.

## Commands

- `cargo build` / `cargo check`
- `cargo run -p kolibri-engine --example window` — spins up the GameWindow (requires GPU)
- `cargo run -p kolibri-engine --example window --features hmr` — enables shader hot-reload (requires GPU + filesystem watcher)
- No test suite exists yet.
- `cargo fmt` (indent = 2 spaces via `.rustfmt.toml`)

## Architecture

- `kolibri-engine/src/wnd_game.rs` — `GameWindow` implements `winit::ApplicationHandler`. Owns the wgpu surface/device/queue and runs a render loop via `ControlFlow::Poll`.
- `kolibri-engine/src/game.rs` — `Game` struct orchestrates the main loop. Behind `#[cfg(hmr)]`, owns an `mpsc::channel` for receiving shader-change events from watcher threads. `Game::update()` drains pending shader-change paths and forwards them to `scene.on_shader_changed()`. `RenderContext` exposes a `shader_tx: Sender<PathBuf>` field behind `#[cfg(hmr)]` so scenes can start watchers during init.
- `kolibri-engine/src/scenes/scene.rs` — `Scene` trait (`init`, `update`, `render`). Scenes are low-level rendering primitives; each implements its own higher-level abstractions internally (e.g. scene graphs, AABB filtering). Implementations vary wildly — a `ProceduralScene` bears little resemblance to a `Scene2D` or `Scene3D`. The trait also provides `fn on_shader_changed(&mut self, ctx: &RenderContext, path: &Path) -> Result<(), EngineError>` with a default no-op, allowing scenes to opt into hot-reload support.
- `kolibri-engine/src/scenes/proc/` — `ProceduralScene`: no entities, no scene graph. Full-screen triangle-strip with WGSL shaders compiled at init. Rendering functions are composed via the `PRF` trait (parser-combinator-style: `evolve` + `compile` to WGSL). Shader source lives in `proc.wgsl`. Constructor is unified across both modes: `ProceduralScene::new(factory: impl FragmentFactory + 'static)`, stores `fragment_factory: Box<dyn FragmentFactory>`. `FragmentFactory` is a trait (not cfg-gated) with `fn create_module(&self, device: &Device) -> Result<ShaderModule, anyhow::Error>`, plus `#[cfg(hmr)] fn watch_path(&self) -> Option<&Path>` (default `None`). `ProceduralScene::on_shader_changed` calls `self.fragment_factory.watch_path()` to check relevance, then `self.fragment_factory.create_module()` to rebuild the shader module and pipeline. `ProceduralSceneState` retains `vx_shader` and `pipeline_layout` for reuse during hot-reload. `start_watcher(&path, &sender)` is a static method that spawns a `notify::RecommendedWatcher` on a background thread, taking the path from the factory trait rather than from self. The `fragment_shader!($path:literal)` macro (`#[macro_export]`, re-exported via prelude) generates a `FragmentFactory` impl: `#[cfg(not(hmr))]` uses `wgpu::include_wgsl!($path)` at compile time; `#[cfg(hmr)]` reads the file at runtime and returns `Some(path)` from `watch_path()`. Usage is identical in both modes: `ProceduralScene::new(fragment_shader!("proc.wgsl"))`.
- `kolibri-engine/src/scenes/proc/fns.rs` — Concrete `PRF` implementations: `Const` (literal WGSL values with proper suffixes — `i`, `u`, `f`, `true`/`false`), `Max`/`Min` (variadic, folds via nested binary `max`/`min` calls; single arg passes through, zero args is an error), `Clamp(low, value, max)` (emits WGSL `clamp`), `BinOp`/`UnaryOp` (operator overloads via `impl_binop!` macro).
- `kolibri-engine/src/scenes/entity2d.rs`, `entity3d.rs` — placeholders (will implement their own scene graphs and filtering heuristics).
- `kolibri-engine/src/error.rs` — `EngineError` enum (thiserror).
- `kolibri-engine/src/camera.rs` — `Camera` trait (projection matrix), not yet wired into scenes.
- `kolibri-engine/src/lib.rs` — re-exports `PRF` and `PRFBox` at the crate root.

## Key details

- `ProceduralScene.render()` draws a hardcoded fullscreen quad (vertex indices 0..4). The fragment shader currently outputs transparent black — the interesting work is TODO.
- `PRF` is a trait (`evolve` + `compile`). `PRFBox` is `Box<dyn PRF>`. Implementors emit WGSL fragment code via `compile()`.
- `PRFBox` has operator overloads (`Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`) defined via `impl_binop!` macro in `proc/mod.rs`, producing `BinOp`/`UnaryOp` nodes.
- `#[kolibri_derive::define(crate::PRF)]` is a proc macro attribute that generates `impl From<Self> for Box<dyn PRF>` for any annotated type, eliminating manual boxing boilerplate. Used on `Const`, `Max`, `Min`, and `Clamp`.
- Convenience macros `max!`, `min!`, `clamp!` (macro_export'd) wrap the `boxed()` helper to convert heterogeneous args into `PRFBox` before constructing variadic nodes.
- GPU init uses `pollster::block_on` on the winit `resumed` callback.
- Uniform buffers use `bytemuck::Pod + Zeroable` with column-major mat4 projection.
