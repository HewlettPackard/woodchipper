// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::{self, JoinHandle};

use serde_json;

use crate::config::Config;
use crate::renderer::types::*;

pub fn json_renderer(_: Arc<Config>, rx: Receiver<LogEntry>) -> JoinHandle<()> {
  thread::Builder::new().name("json_renderer".to_string()).spawn(move || {
    for entry in rx {
      if entry.eof.is_some() {
        break;
      }

      if let Some(message) = entry.message {
        match serde_json::to_string(&message.message) {
          Ok(s) => println!("{}", s),
          Err(e) => {
            eprintln!("error converting message to json: {:?}", e);
            break;
          }
        }
      }
    }
  }).unwrap()
}
