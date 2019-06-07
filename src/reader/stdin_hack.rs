// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use simple_error::{SimpleError, SimpleResult};

use crate::config::Config;
use crate::renderer::LogEntry;

/// reads the process stdin directly using Evil Hacks to ensure our fd doesn't
/// get closed when the interactive UI opens /dev/tty
/// for some reason opening /dev/tty closes our stdin pipe even though it still
/// technically remains readable at /dev/stdin
/// note that this _probably_ only works on linux, other OSes may need to run
/// their application via a subprocess or just fall back to the styled renderer
pub fn read_stdin_hack(
  _config: Arc<Config>,
  tx: Sender<LogEntry>,
  _exit_req_rx: Receiver<()>,
  _exit_resp_tx: Sender<()>
) -> JoinHandle<SimpleResult<()>> {
  thread::Builder::new().name("read_stdin_hack".to_string()).spawn(move || {
    let file = File::open("/dev/stdin").map_err(SimpleError::from)?;

    let mut empty = true;
    for line in BufReader::new(file).lines() {
      let line = line.map_err(SimpleError::from)?;
      empty = false;

      match LogEntry::message(&line, None) {
        Ok(Some(entry)) => match tx.send(entry) {
          Ok(_) => (),
          Err(_) => break
        },
        Err(_) => continue,
        _ => continue
      }
    }

    if empty {
      tx.send(LogEntry::internal(
        "warning: reached end of input without reading any messages"
      )).ok();
    }

    // not much we can do if this fails
    tx.send(LogEntry::eof()).ok();

    Ok(())
  }).unwrap()
}

