// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

//#![warn(clippy)]

extern crate atty;
extern crate chrono;
#[cfg(not(target_os = "linux"))] extern crate clipboard;
extern crate crossterm;
extern crate dtparse;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate maplit;
extern crate rand;
extern crate regex;
extern crate shellexpand;
#[macro_use] extern crate simple_error;
extern crate structopt;
extern crate subprocess;

use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use structopt::StructOpt;

mod config;
mod clip;
mod filter;
mod style;
mod reader;
mod parser;
mod classifier;
mod renderer;

use config::Config;

// re: panic when piping to e.g. head
//https://github.com/rust-lang/rust/issues/24821#issuecomment-96276655

fn main() -> Result<(), Box<Error>> {
  let config = Arc::new(Config::from_args());

  let (entry_tx, entry_rx) = channel();

  let renderer_impl = config.renderer.get_renderer(Arc::clone(&config));
  let renderer = renderer_impl(Arc::clone(&config), entry_rx);

  // kick off the reader thread and hope it goes on to do great things
  // due to blocking IO limitations we can't ever expect to actually get a
  // result out of it, and will have to let the OS handle cleanup for us
  let (exit_req_tx, exit_req_rx) = channel();
  let (exit_resp_tx, exit_resp_rx) = channel();

  let reader_impl = config.reader.get_reader(Arc::clone(&config));
  reader_impl(
    Arc::clone(&config),
    entry_tx,
    exit_req_rx, exit_resp_tx
  );

  renderer.join().expect("renderer thread did not exit cleanly");
  
  // attempt to tell the reader to quit (though it'll probably be ignored)
  exit_req_tx.send(()).ok();

  // and wait at most 1s for an exit confirmation
  exit_resp_rx.recv_timeout(Duration::from_millis(1000)).ok();

  Ok(())
}
