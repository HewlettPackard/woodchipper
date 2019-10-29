// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossterm::{Crossterm, Screen, TerminalInput, InputEvent};

use crate::config::Config;
use crate::renderer::types::*;

pub mod state;
pub mod text;
pub mod log;
pub mod bar;
pub mod status_bar;
pub mod filter_bar;
pub mod search_bar;

pub use state::RenderState;
pub use state::RcState;

lazy_static! {
  /// The interval between full redraws even if no inputs occur
  static ref REFRESH_INTERVAL: Duration = Duration::from_millis(500);
}

#[derive(PartialEq)]
pub enum InputAction {
  /// The application should exit
  Exit,
  
  /// The input was handled and the screen should be re-rendered
  Rerender,

  /// No action / other handlers may override
  Unhandled
}

pub fn interactive_renderer(config: Arc<Config>, rx: Receiver<LogEntry>) -> JoinHandle<()> {
  thread::Builder::new().name("interactive".to_string()).spawn(move || {
    let mut rs = Rc::new(RenderState::new(Arc::clone(&config)));

    let screen = Screen::default();
    let alt = match screen.enable_alternate_modes(true) {
      Ok(alternate) => alternate,
      Err(e) => {
        eprintln!("error opening alternate mode: {:?}", e);
        return;
      }
    };

    let sleep_duration_seconds = 1.0f32 / &config.refresh_hz;

    let crossterm = Crossterm::from_screen(&alt.screen);
    let cursor = crossterm.cursor();
    let terminal = crossterm.terminal();

    let input = TerminalInput::from_output(&alt.screen.stdout);

    let mut stdin = input.read_async();

    let mut last_render: Option<Instant> = None;
    let (mut last_width, mut last_height) = (0, 0);
    'outer: loop {
      // whether or not we should redraw at the end of this iter
      let mut dirty = false;

      for entry in rx.try_iter() {
        if let Some(message) = entry.message {
          rs = state::actions::add_entry(rs.clone(), message);
          dirty = true;
        }

        if entry.eof.is_some() {
          rs = state::actions::set_eof(rs.clone(), true);
          dirty = true;
        }
      }

      // handle as many input events as we can
      while let Some(event) = stdin.next() {
        if let InputEvent::Keyboard(key) = event {
          let (new_state, action) = bar::input(rs.clone(), key);
          rs = new_state;

          match action {
            InputAction::Exit => break 'outer,
            InputAction::Rerender => dirty = true,
            InputAction::Unhandled => ()
          };
        };
      }

      // TODO: is calling terminal_size() every loop expensive?
      let (width, height) = terminal.terminal_size();
      let resized = (width != last_width) || (height != last_height);
      if resized {
        let rs_mut = Rc::make_mut(&mut rs);
        rs_mut.width = width;
        rs_mut.height = height;
      }

      last_width = width;
      last_height = height;

      // TODO: crossterm doesn't seem to support resize events yet
      // until then, just rerender occasionally
      // resizes won't be very smooth, but it will clean itself up
      // note that we also want to reduce unnecessary redraws as they can
      // clear a user's terminal selection
      let force_refresh = if let Some(last_render) = last_render {
        resized && (last_render.elapsed() >= *REFRESH_INTERVAL)
      } else {
        // first render
        true
      };

      if dirty || force_refresh {
        // TODO actually render
        rs = log::render(rs.clone(), &terminal, &cursor).unwrap();
        rs = bar::render(rs.clone(), &terminal, &cursor).unwrap();

        last_render = Some(Instant::now());
      }

      thread::sleep(Duration::from_secs_f32(sleep_duration_seconds));
    }

    // attempt to un-hide the cursor on the way out
    cursor.show().ok();
  }).unwrap()
}
