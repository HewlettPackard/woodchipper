// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use chrono::prelude::*;
use dtparse::Parser;
use regex::RegexSet;

use crate::config::Config;
use super::types::{LogLevel, Message, MessageKind, ReaderMetadata};
use super::util::normalize_datetime;

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

/// uses wild guesses to ignore false positive timestamps
/// dtparse does a pretty good job detecting timestamps when they're actually
/// present in the string, but almost always returns something when strings
/// _don't_ actually contain a timestamp
fn crappy_is_false_positive(line: &str, maybe_tokens: Option<Vec<String>>) -> bool {
  let tokens = if let Some(tokens) = maybe_tokens {
    tokens
  } else {
    return false;
  };

  let len_tokens: usize = tokens.iter().map(|t| t.chars().count()).sum();
  let len_line: i32 = line.chars().count() as i32;
  let len_consumed_tokens: i32 = len_line - len_tokens as i32;

  // dtparse detects "" as a timestamp :(
  if len_line == 0 {
    return true;
  }

  // probably can't generate a timestamp in fewer characters than a timestamp
  // (assuming an unhypenated iso8601 w/ missing timezone specifier)
  if len_consumed_tokens < 15 {
    return true;
  }

  // arbitrary, but throw away anything extremely long
  if len_consumed_tokens > 60 {
    return true;
  }

  // in theory a timestamp should make up a small portion of each log line
  // so, throw away timestamps if they account for more than 75% of the line
  let timestampiness = len_consumed_tokens as f32 / len_line as f32;
  if timestampiness > 0.75 {
    return true;
  }

  false
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
  let timestamp = if let Some(timestamp) = get_meta_timestamp(&meta) {
    Some(timestamp)
  } else {
    // TODO: try to remove timestamp from text?
    // TODO: tzinfos would be nice to detect named timezones...
    let parser = Parser::default();

    let parse_result = parser.parse(
      line,
      None, // dayfirst
      None, // yearfirst
      true, // fuzzy
      true, // fuzzy_with_tokens
      None, // default
      false, // ignoretz
      &HashMap::new() // tzinfos
    );

    match parse_result {
      Ok((datetime, offset, tokens)) => {
        if crappy_is_false_positive(line, tokens) {
          None
        } else {
          Some(normalize_datetime(&datetime, offset))
        }
      },
      Err(_) => None
    }
  };

  Ok(Some(Message {
    kind: MessageKind::Plain,
    timestamp,
    level: get_log_level(line),
    text: Some(String::from(line)),
    metadata: HashMap::new(),
    reader_metadata: meta,
    mapped_fields: HashMap::new()
  }))
}
