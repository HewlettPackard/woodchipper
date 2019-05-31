// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashMap;
use std::error::Error;

use chrono::prelude::*;
use regex::Regex;
use serde_json::{self, Value, Map};

use super::types::{
  LogLevel, MappingField, Message, MessageKind, ReaderMetadata
};
use super::util::{parse_timestamp, normalize_datetime};

static TIMESTAMP_FIELDS: &[&str] = &["timestamp", "@timestamp", "time"];
static LEVEL_FIELDS: &[&str] = &["level"];
static TEXT_FIELDS: &[&str] = &["text", "msg", "message"];

pub fn get_value<'a, 'b>(
  map: &'b Map<String, Value>,
  key_choices: &[&'a str]
) -> Option<(&'a str, &'b Value)> {
  for key in key_choices {
    if let Some(val) = map.get(*key) {
      return Some((key, &val));
    }
  }

  None
}

/// determines if the date string is a simple RFC-2822 datetime, and if so,
/// parses it
/// we use dtparse to parse more free-form dates, but its parser is surprisingly
/// expensive. as most structured logs will use some form of iso8601, we can try
/// to use chrono's built in and much cheaper parser to save some cycles
pub fn parse_rfc2822(s: &str) -> Option<DateTime<Utc>> {
  lazy_static! {
    static ref RE: Regex = Regex::new(
      r"\w+, \d+ \w+ \d{4} \d{2}:\d{2}:\d{2} (?:UTC|\+\d{4})"
    ).unwrap();
  }

  if RE.is_match(s) {
    match DateTime::parse_from_rfc2822(s) {
      Ok(d) => Some(normalize_datetime(&d.naive_local(), Some(*d.offset()))),
      Err(_) => None
    }
  } else {
    None
  }
}

/// determines if the date string is a simple RFC-3339 datetime, and if so,
/// parses it
/// we use dtparse to parse more free-form dates, but its parser is surprisingly
/// expensive. as most structured logs will use some form of iso8601, we can try
/// to use chrono's built in and much cheaper parser to save some cycles
pub fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
  lazy_static! {
    static ref RE: Regex = Regex::new(
      r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::[\d.]+)?(?:Z|-\d{2}:\d{2})"
    ).unwrap();
  }

  if RE.is_match(s) {
    match DateTime::parse_from_rfc3339(s) {
      Ok(d) => Some(normalize_datetime(&d.naive_local(), Some(*d.offset()))),
      Err(_) => None
    }
  } else {
    None
  }
}

pub fn parse_freeform(s: &str) -> Option<DateTime<Utc>> {
  match parse_timestamp(s) {
    Ok((datetime, offset)) => Some(normalize_datetime(&datetime, offset)),
    Err(_) => None
  }
}

/// Extract the timestamp from any supported field in the message, returning
/// both the field and the parsed NaiveDateTime
pub fn get_timestamp(msg: &Map<String, Value>) -> Option<(&str, DateTime<Utc>)> {
  if let Some((k, v)) = get_value(&msg, TIMESTAMP_FIELDS) {
    let v_str = if let Some(v) = v.as_str() {
      v
    } else {
      return None;
    };

    parse_rfc3339(v_str)
      .or_else(|| parse_rfc2822(v_str))
      .or_else(|| parse_freeform(v_str))
      .and_then(|dt| Some((k, dt)))
  } else {
    None
  }
}

pub fn parse_json(
  line: &str, meta: Option<ReaderMetadata>
) -> Result<Option<Message>, Box<Error>> {
  // skip anything that doesn't at least vaguely look like json
  if !line.starts_with('{') || !line.ends_with('}') {
    return Ok(None);
  }

  let mut mapped_fields = HashMap::new();

  let msg: Map<String, Value> = match serde_json::from_str(line) {
    Ok(message) => message,
    Err(_) => return Ok(None)
  };

  let timestamp = if let Some((key, timestamp)) = get_timestamp(&msg) {
    mapped_fields.insert(String::from(key), MappingField::Timestamp);
    Some(timestamp)
  } else {
    None
  };

  let level = if let Some((key, value)) = get_value(&msg, LEVEL_FIELDS) {
    if let Some(level) = value.as_str().and_then(|s| s.parse::<LogLevel>().ok()) {
      mapped_fields.insert(String::from(key), MappingField::Level);
      Some(level)
    } else {
      None
    }
  } else {
    None
  };

  let text = if let Some((key, text)) = get_value(&msg, TEXT_FIELDS) {
    if let Some(text) = text.as_str() {
      mapped_fields.insert(String::from(key), MappingField::Text);

      let trimmed = text.trim();
      if trimmed.is_empty() {
        None
      } else {
        Some(trimmed.to_string())
      }
    } else {
      None
    }
  } else {
    None
  };

  // clone remaining fields into the message metadata
  let metadata: HashMap<String, Value> = msg.iter()
    .filter(|(k, _v)| !mapped_fields.contains_key(k.as_str()))
    .map(|(k, v)| (k.to_string(), v.to_owned()))
    .collect();

  let message = Message {
    kind: MessageKind::Json,
    reader_metadata: meta,
    timestamp, level, text, metadata, mapped_fields
  };

  Ok(Some(message))
}
