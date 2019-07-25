// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use chrono::prelude::*;
use regex::RegexSet;

use crate::config::Config;
use super::types::{LogLevel, Message, MessageKind, ReaderMetadata};

fn get_log_level(line: &str) -> Option<LogLevel> {
  lazy_static! {
    static ref REGEXES: RegexSet = RegexSet::new(&[
      r"(?i)\bfatal\b", // FATAL, fatal
      r"(?i)\berr(?:or)?\b", // ERR, ERROR, err, error
      r"(?i)\bwarn(?:ing)?\b", // WARN, WARNING, warn, warning
      r"(?i)\binfo\b", // INFO, info
      r"(?i)\b(?:debug|dbg)\b", // DBG, DEBUG, dbg, debug
    ]).unwrap();
  }

  // a bit of a strange structure, but we only really expect to have one match
  // in a line (and will try to return that first match if there are multiple).
  // this does allow us to define multiple regexes per level in the set and add
  // them here
  for index in REGEXES.matches(line).iter() {
    return match index {
      0 => Some(LogLevel::Fatal),
      1 => Some(LogLevel::Error),
      2 => Some(LogLevel::Warning),
      3 => Some(LogLevel::Info),
      4 => Some(LogLevel::Debug),
      _ => continue
    };
  }

  None
}

fn get_meta_timestamp(meta: &Option<ReaderMetadata>) -> Option<DateTime<Utc>> {
  if let Some(meta) = meta {
    if let Some(timestamp) = meta.timestamp {
      return Some(timestamp);
    }
  }

  None
}

pub fn parse_plain(
  _config: Arc<Config>, line: &str, meta: Option<ReaderMetadata>
) -> Result<Option<Message>, Box<Error>> {
  Ok(Some(Message {
    kind: MessageKind::Plain,
    timestamp: get_meta_timestamp(&meta),
    level: get_log_level(line),
    text: Some(String::from(line)),
    metadata: HashMap::new(),
    reader_metadata: meta,
    mapped_fields: HashMap::new()
  }))
}
