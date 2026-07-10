use std::fmt::Display;

use kolibri_engine::prelude::*;

#[derive(Debug, Clone, Default)]
enum GameState {
  #[default]
  Initial,
  Main,
}

fn main() {
  let mut state = GameState::Initial;

  Game::run(move || {
    match state {
      GameState::Initial => {
        state = GameState::Main;
        Ok(Some(Box::new(ProceduralScene::new(frag!("cube.wgsl")))))
      }
      _ => {
        Ok(None) // No scene switch
      }
    }
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
