// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::{self, JoinHandle};

use crate::config::Config;
use crate::renderer::types::*;

pub fn raw_renderer(_: Arc<Config>, rx: Receiver<LogEntry>) -> JoinHandle<()> {
  thread::Builder::new().name("raw_renderer".to_string()).spawn(move || {
    for entry in rx {
      if entry.eof.is_some() {
        break;
      }

      if let Some(message) = entry.message {
        println!("{}", message.message.raw);
      }
    }
  }).unwrap()
}
