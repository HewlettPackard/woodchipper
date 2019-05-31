// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::{self, JoinHandle};

use crossterm::Crossterm;

use crate::config::Config;
use crate::renderer::types::*;
use crate::renderer::common::*;

/// A container for one or more wrapped lines in a message
#[derive(Debug, Clone)]
pub struct RenderedMessage {
  lines: Vec<RenderedChunk>
}

pub fn styled_renderer(config: Arc<Config>, rx: Receiver<LogEntry>) -> JoinHandle<()> {
  thread::Builder::new().name("styled_renderer".to_string()).spawn(move || {
    let screen = Crossterm::new();
    let term = screen.terminal();

    let profile = &config.style.normal;

    for entry in rx {
      if let Some(message_entry) = entry.message {
        let term_width = match term.terminal_size().0 as usize {
          0 => Some(config.fallback_width),
          width => Some(width)
        };

        for line in styled_render(&message_entry, &profile, term_width) {
          println!("{}", line);
        }
      }

      if entry.eof.is_some() {
        break;
      }
    }
  }).unwrap()
}
