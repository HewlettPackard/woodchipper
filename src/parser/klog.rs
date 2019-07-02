// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use chrono::prelude::*;
use regex::Regex;
use serde_json::Value;

use crate::config::Config;
use super::types::{LogLevel, Message, MessageKind, ReaderMetadata};

fn map_klog_level(level: &str) -> Option<LogLevel> {
  match level {
    "D" => Some(LogLevel::Debug), // not technically supported by klog
    "I" => Some(LogLevel::Info),
    "W" => Some(LogLevel::Warning),
    "E" => Some(LogLevel::Error),
    "F" => Some(LogLevel::Fatal),
    _ => None
  }
}

// parses klog-style messages
//
// based on the format description at:
// https://github.com/kubernetes/klog/blob/master/klog.go#L592-L602
pub fn parse_klog(
  _config: Arc<Config>, line: &str, meta: Option<ReaderMetadata>
) -> Result<Option<Message>, Box<Error>> {
  lazy_static! {
    static ref RE: Regex = Regex::new(
      r"^([A-Z])(\d{4} \d{2}:\d{2}:[\d\.]+)\s+(\d+) ([\S.]+:\d+)] (.+)$"
    ).unwrap();
  }

  if let Some(caps) = RE.captures(line) {
    // naughty unwrapping, but we have a constant number of groups
    let level = map_klog_level(caps.get(1).unwrap().as_str());

    let timestamp_str = caps.get(2).unwrap().as_str();
    
    // ex: 0607 19:28:33.579841
    let reader_timestamp = if let Some(meta) = &meta {
      if let Some(timestamp) = &meta.timestamp {
        Some(*timestamp)
      } else {
        None
      }
    } else { None };

    let timestamp = Utc.datetime_from_str(
      timestamp_str,
      "%m%d %H:%M:%S:%.f"
    ).ok().or(reader_timestamp);
    let text = caps.get(5).unwrap().as_str();

    let mut metadata = HashMap::new();

    let maybe_thread_id = caps.get(3)
      .map(|c| c.as_str())
      .and_then(|s| s.parse::<isize>().ok());
    if let Some(thread_id) = maybe_thread_id {
      metadata.insert("threadId".to_string(), Value::Number(thread_id.into()));
    }

    if let Some(context) = caps.get(4).map(|c| c.as_str()) {
      metadata.insert(
        "caller".to_string(),
        Value::String(context.to_string())
      );
    }

    return Ok(Some(Message {
      kind: MessageKind::Klog,
      reader_metadata: meta,
      text: Some(text.to_string()),

      timestamp, level, metadata,

      mapped_fields: hashmap!{}
    }));
  }

  Ok(None)
}
