// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::error::Error;
use std::sync::Arc;

use pest::Parser;
use serde_json::{self, Value, Map};
use simple_error::SimpleError;

use crate::config::Config;
use super::types::{Message, ReaderMetadata};
use super::json::parse_document;

#[derive(Parser)]
#[grammar = "parser/logrus.pest"]
struct LogrusParser;

/// Converts logrus-style plaintext into a JSON document
///
/// Logrus plaintext output is of the form:
///     time="2015-03-26T01:27:38-04:00" level=debug msg="hello world" foo="bar"
///
/// This uses a custom pest parser to convert it to a JSON-like document so the
/// regular JSON parser can then parse it normally.
pub fn logrus_to_document(
  line: &str
) -> Result<Map<String, Value>, Box<Error>> {
  let mut pairs = LogrusParser::parse(Rule::logrus, line)?;
  let logrus = pairs.next().ok_or_else(
    || SimpleError::new("unparsable logrus line")
  )?;

  let mut doc = Map::new();

  for pair in logrus.into_inner() {
    let mut key: Option<String> = None;
    let mut value: Option<Value> = None;

    for inner in pair.into_inner() {
      match inner.as_rule() {
        Rule::key => key = Some(inner.as_str().to_string()),
        Rule::string | Rule::bare_string | Rule::object => {
          let s = inner.as_str();

          value = if s == "true" {
            Some(Value::Bool(true))
          } else if s == "false" {
            Some(Value::Bool(false))
          } else if let Ok(int) = s.parse::<i64>() {
            Some(Value::Number(int.into()))
          } else {
            Some(Value::String(inner.as_str().to_string()))
          };
        },
        Rule::EOI => (),
        _ => unreachable!()
      }
    }

    if let (Some(key), Some(value)) = (key, value) {
      doc.insert(key, value);
    }
  }

  Ok(doc)
}

pub fn parse_logrus(
  _config: Arc<Config>, line: &str, meta: Option<ReaderMetadata>
) -> Result<Option<Message>, Box<Error>> {
  match logrus_to_document(line) {
    Ok(doc) => parse_document(doc, meta),
    Err(_) => Ok(None)
  }
}
