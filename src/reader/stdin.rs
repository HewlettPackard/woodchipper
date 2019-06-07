// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::io::{self, BufRead};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use simple_error::{SimpleError, SimpleResult};

use crate::config::Config;
use crate::renderer::LogEntry;

// TODO: if we want to surface errors, it might be best to send it as a message
// over the tx channel
// reader threads will not be killed due to the apparent lack of non-blocking
// file IO, so if readers are still running we'll just exit and let the OS deal
// with them
// (tokio may help here, but it's unclear to me if tokio can do async reads by
// line rather than reading the whole file into memory... which is obviously
// wrong for our use case)

pub fn read_stdin(
  _config: Arc<Config>,
  tx: Sender<LogEntry>,
  _exit_req_rx: Receiver<()>,
  _exit_resp_tx: Sender<()>
) -> JoinHandle<SimpleResult<()>> {
  // for some reason this dies when crossterm opens /dev/tty
  // the hack reader works around this issue but may have compatibility
  // problems

  thread::Builder::new().name("read_stdin".to_string()).spawn(move || {
    let mut empty = true;
    for line in io::stdin().lock().lines() {
      let line = line.map_err(SimpleError::from)?;
      empty = false;

      match LogEntry::message(&line, None) {
        Ok(Some(entry)) => match tx.send(entry) {
          Ok(_) => (),
          // assume receiver has quit and stop
          Err(_) => break
        },
        Err(_) => continue,
        _ => continue
      };
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
