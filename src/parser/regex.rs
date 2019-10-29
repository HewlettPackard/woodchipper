// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::iter::FromIterator;
use std::sync::Arc;

use chrono::prelude::*;
use serde_json::Value;

use crate::config::{Config, RegexMapping};
use super::types::{LogLevel, Message, MessageKind, ReaderMetadata};
use super::util::normalize_datetime;

#[cfg(test)] use spectral::prelude::*;

fn parse_rfc2822(s: &str) -> Option<DateTime<Utc>> {
  match DateTime::parse_from_rfc2822(s) {
    Ok(d) => Some(normalize_datetime(&d.naive_local(), Some(*d.offset()))),
    Err(_) => None
  }
}

fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
  match DateTime::parse_from_rfc3339(s) {
    Ok(d) => Some(normalize_datetime(&d.naive_local(), Some(*d.offset()))),
    Err(_) => None
  }
}

fn parse_format(
  s: &str, fmt: &str, prepend: &Option<String>
) -> Option<DateTime<Utc>> {
  let datetime = if let Some(prepend) = prepend {
    format!(
      "{} {}",
      Utc::now().format(prepend),
      s
    )
  } else {
    String::from(s)
  };

  Utc.datetime_from_str(&datetime, fmt).ok()
}

fn parse_datetime(
  fmt: &str, datetime: &str, prepend: &Option<String>
) -> Option<DateTime<Utc>> {
  match fmt {
    "rfc2822" => parse_rfc2822(datetime),
    "rfc3339" => parse_rfc3339(datetime),
    _ => parse_format(datetime, fmt, prepend)
  }
}

fn parse_mapping(
  line: &str, mapping: &RegexMapping, meta: &Option<ReaderMetadata>
) -> Result<Option<Message>, Box<dyn Error>> {
  let caps = match mapping.pattern.captures(line) {
    Some(caps) => caps,
    None => return Ok(None)
  };

  let mut group_names: HashSet<String> = HashSet::from_iter(
    mapping.pattern.capture_names().filter_map(|n| n.map(String::from))
  );

  let timestamp = if let Some(datetime) = caps.name("datetime") {
    if let Some(format) = &mapping.datetime {
      group_names.remove("datetime");

      parse_datetime(&format, datetime.as_str(), &mapping.datetime_prepend)
    } else {
      None
    }
  } else {
    None
  };

  let text = if let Some(text) = caps.name("text") {
    group_names.remove("text");

    Some(String::from(text.as_str()))
  } else {
    None
  };

  let level = if let Some(level) = caps.name("level") {
    group_names.remove("level");

    match level.as_str().parse::<LogLevel>() {
      Ok(l) => Some(l),
      Err(_) => None
    }
  } else {
    None
  };

  // collect all other capture groups into the metadata
  let mut metadata = HashMap::new();
  for name in group_names {
    if let Some(mat) = caps.name(&name) {
      metadata.insert(
        name,
        Value::String(String::from(mat.as_str()))
      );
    }
  }

  let message = Message {
    kind: MessageKind::Regex,
    reader_metadata: meta.clone(),
    raw: line.to_string(),
    timestamp, level, text, metadata,
    mapped_fields: HashMap::new()
  };

  Ok(Some(message))
}

/// attempts to parse a line using one or more user-specified regexes with named
/// capture groups
pub fn parse_regex(
  config: Arc<Config>,
  line: &str, meta: Option<ReaderMetadata>
) -> Result<Option<Message>, Box<dyn Error>> {
  if let Some(regexes) = &config.regexes {
    for mapping in &regexes.mappings {
      match parse_mapping(line, mapping, &meta) {
        Ok(Some(message)) => return Ok(Some(message)),
        Ok(None) => continue,
        Err(e) => return Err(e)
      };
    }
  }
  
  Ok(None)
}

#[cfg(test)]
mod tests {
  use super::*;

  use regex::Regex;
  use serde_json::json;
  use simple_error::{SimpleResult, SimpleError};

  fn mapping(pattern: &str, datetime: &str) -> RegexMapping {
    RegexMapping {
      pattern: Regex::new(pattern).unwrap(),
      datetime: Some(String::from(datetime)),
      datetime_prepend: None
    }
  }

  fn parse_to_value(
    line: &str, mapping: &RegexMapping, meta: &Option<ReaderMetadata>
  ) -> SimpleResult<Value> {
    let parsed = parse_mapping(line, mapping, meta)
      .map_err(|e| SimpleError::new(format!("{:?}", e)))?;

    serde_json::to_value(parsed).map_err(SimpleError::from)
  }

  #[test]
  fn test_empty() {
    let value = parse_to_value(
      "",
      &mapping(r"", "rfc3339"),
      &None
    );

    assert_that!(value).is_ok_containing(json!({
      "kind": "regex"
    }));
  }

  #[test]
  fn test_only_rfc3339() {
    let value = parse_to_value(
      "2019-10-01T20:40:49Z",
      &mapping(r"^(?P<datetime>.+)$", "rfc3339"),
      &None
    );

    assert_that!(value).is_ok_containing(json!({
      "kind": "regex",
      "timestamp": "2019-10-01T20:40:49Z"
    }));
  }

  #[test]
  fn test_only_rfc2822() {
    let value = parse_to_value(
      "Tue, 1 Jul 2003 10:52:37 +0200",
      &mapping(r"^(?P<datetime>.+)$", "rfc2822"),
      &None
    );

    // input dates are normalized to rfc3339 and utc
    assert_that!(value).is_ok_containing(json!({
      "kind": "regex",
      "timestamp": "2003-07-01T08:52:37Z"
    }));
  }

  #[test]
  fn test_metadata() {
    let value = parse_to_value(
      "foo bar",
      &mapping(r"^(?P<a>\S+) (?P<b>\S+)$", "rfc2822"),
      &None
    );

    assert_that!(value).is_ok_containing(json!({
      "kind": "regex",
      "metadata": {
        "a": "foo",
        "b": "bar"
      }
    }));
  }

  #[test]
  fn test_invalid_date() {
    let value = parse_to_value(
      "2019-10-01T20:40:49Z",
      &mapping(r"^(?P<datetime>.+)$", "rfc2822"),
      &None
    );

    // invalid date should be null -> not included in json doc
    assert_that!(value).is_ok_containing(json!({
      "kind": "regex"
    }));
  }

  #[test]
  fn test_full_klog() {
    let mapping = RegexMapping {
      pattern: Regex::new(&format!(
        r"^{}{}\s+{} {}] {}$",
        r"(?P<level>[A-Z])",
        r"(?P<datetime>\d{4} \d{2}:\d{2}:[\d\.]+)",
        r"(?P<threadId>\d+)",
        r"(?P<file>[\S.]+:\d+)",
        r"(?P<text>.+)"
      )).unwrap(),
      datetime: Some(String::from("%Y %m%d %H:%M:%S%.f")),
      datetime_prepend: Some(String::from("%Y"))
    };

    let value = parse_to_value(
      "I0703 17:19:11.688460       1 controller.go:293] hello world",
      &mapping,
      &None
    );

    assert_that!(value).is_ok_containing(json!({
      "kind": "regex",
      "level": "info",
      "text": "hello world",
      "timestamp": "2019-07-03T17:19:11.688460Z",
      "metadata": {
        "threadId": "1",
        "file": "controller.go:293"
      }
    }));
  }

  #[test]
  fn test_klog_invalid() {
    let mapping = RegexMapping {
      pattern: Regex::new(&format!(
        r"^{}{}\s+{} {}] {}$",
        r"(?P<level>[A-Z])",
        r"(?P<datetime>\d{4} \d{2}:\d{2}:[\d\.]+)",
        r"(?P<threadId>\d+)",
        r"(?P<file>[\S.]+:\d+)",
        r"(?P<text>.+)"
      )).unwrap(),
      datetime: Some(String::from("%Y %m%d %H:%M:%S%.f")),
      datetime_prepend: Some(String::from("%Y"))
    };

    let value = parse_to_value(
      "foo",
      &mapping,
      &None
    );

    assert_that!(value).is_ok_containing(json!(null));
  }

  #[test]
  fn test_full_docs_example() {
    let value = parse_to_value(
      "2019-07-03 12:02:13,977 - DEBUG    - test.py:9 - this is a debug message",
      &mapping(&format!(
        "^{} - {} - {} - {}$",
        r"^(?P<datetime>[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2})(?:,[0-9]+)",
        r"(?P<level>\w+)\s*",
        r"(?P<file>\S+)\s*",
        r"(?P<text>.+)$"
      ), "%Y-%m-%d %H:%M:%S"),
      &None
    );

    assert_that!(value).is_ok_containing(json!({
      "kind": "regex",
      "level": "debug",
      "text": "this is a debug message",
      "timestamp": "2019-07-03T12:02:13Z",
      "metadata": {
        "file": "test.py:9"
      }
    }));
  }
}
