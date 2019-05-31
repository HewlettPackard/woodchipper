// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use simple_error::SimpleError;

use crate::config::Config;
use crate::renderer::LogEntry;

pub type Reader = fn(
  config: Arc<Config>, tx: Sender<LogEntry>,
  exit_req_rx: Receiver<()>, exit_resp_tx: Sender<()>
) -> JoinHandle<Result<(), SimpleError>>;

