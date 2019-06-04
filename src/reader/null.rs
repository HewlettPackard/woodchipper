// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use simple_error::SimpleResult;

use crate::config::Config;
use crate::renderer::LogEntry;

/// A simple reader to display an error if autodetection fails
pub fn read_null(
  _config: Arc<Config>,
  tx: Sender<LogEntry>,
  _exit_req_rx: Receiver<()>,
  _exit_resp_tx: Sender<()>
) -> JoinHandle<SimpleResult<()>> {
  thread::Builder::new().name("read_null".to_string()).spawn(move || {
    tx.send(LogEntry::internal(
      "error: no reader was detected automatically, either select a reader \
      (e.g. -r kubernetes) or pipe in some input"
    )).ok();

    tx.send(LogEntry::internal(
      "error: see woodchipper --help for details"
    )).ok();

    tx.send(LogEntry::eof()).ok();

    Ok(())
  }).unwrap()
}
