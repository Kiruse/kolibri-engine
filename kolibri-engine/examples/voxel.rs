use std::fmt::Display;

use kolibri_engine::prelude::*;

fn main() {
  let mut initialized = false;

  Game::run(move || {
    if initialized {
      return Ok(None);
    }
    initialized = true;
    let scene = VoxelScene::default();
    // TODO: populate octree
    return Ok(Some(Box::new(scene)));
  }).assert();
}

trait Assert<T> {
  fn assert(self) -> T;
}

impl<T, E: Display> Assert<T> for Result<T, E> {
  fn assert(self) -> T {
    match self {
      Ok(res) => res,
      Err(e) => {
        eprintln!("Error: {e}");
        std::process::exit(1);
      }
    }
  }
}
