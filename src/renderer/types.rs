// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use chrono::offset::Utc;

use crate::config::Config;
use crate::parser::{LogLevel, Message, MessageKind, ReaderMetadata, parse};
use crate::classifier::{Chunk, classify};

#[derive(Debug, Clone)]
pub struct MessageEntry {
  pub message: Message,
  pub chunks: Vec<Chunk>,
}

impl MessageEntry {
  /// creates an internal log message, e.g. to inform the user of an internal
  /// error
  pub fn internal(message: &str) -> MessageEntry {
    let m = Message {
      kind: MessageKind::Internal,
      timestamp: Some(Utc::now()),
      level: Some(LogLevel::Int),
      text: Some(message.to_string()),
      metadata: HashMap::new(),
      reader_metadata: None,
      mapped_fields: HashMap::new(),
    };

    let chunks = classify(&m);

    MessageEntry {
      message: m,
      chunks
    }
  }
}

/// A LogEntry sent when the end of input is reached
#[derive(Debug)]
pub struct EofEntry;

#[derive(Debug)]
pub struct LogEntry {
  pub message: Option<MessageEntry>,
  pub eof: Option<EofEntry>
}

impl Default for LogEntry {
  fn default() -> LogEntry {
    LogEntry {
      message: None,
      eof: None
    }
  }
}

impl LogEntry {
  pub fn eof() -> LogEntry {
    LogEntry {
      eof: Some(EofEntry {}),
      ..Default::default()
    }
  }

  pub fn message(
    config: Arc<Config>, line: &str, meta: Option<ReaderMetadata>
  ) -> Result<Option<LogEntry>, Box<Error>> {
    let message = match parse(config, &line, meta)? {
      Some(message) => message,
      None => return Ok(None)
    };

    let chunks = classify(&message);

    Ok(Some(LogEntry {
      message: Some(MessageEntry { message, chunks }),

      ..Default::default()
    }))
  }

  pub fn internal(message: &str) -> LogEntry {
    LogEntry {
      message: Some(MessageEntry::internal(message)),

      ..Default::default()
    }
  }
}

pub type Renderer = fn(config: Arc<Config>, rx: Receiver<LogEntry>) -> JoinHandle<()>;
