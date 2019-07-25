// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::error::Error;
use std::sync::Arc;

use pest::Parser;
use serde_json::{self, Value, Map};
use simple_error::SimpleError;

use crate::config::Config;
use super::types::{Message, MessageKind, ReaderMetadata};
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
    Ok(doc) => {
      if doc.is_empty() {
        Ok(None)
      } else {
        parse_document(MessageKind::Logrus, doc, meta)
      }
    },
    Err(_) => Ok(None)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::json;
  use simple_error::{SimpleResult, SimpleError};
  use spectral::prelude::*;
  use structopt::StructOpt;

  fn parse(line: &str) -> SimpleResult<Value> {
    let doc = logrus_to_document(line)
      .map_err(|e| SimpleError::new(format!("{:?}", e)))?;

    Ok(Value::Object(doc))
  }

  fn parse_message(line: &str) -> SimpleResult<Value> {
    let config = Arc::new(Config::from_iter_safe(vec![""]).unwrap());
    let parsed = parse_logrus(config, line, None)
      .map_err(|e| SimpleError::new(format!("{:?}", e)))?;

    serde_json::to_value(parsed).map_err(SimpleError::from)
  }

  #[test]
  fn test_empty() {
    // we should get an empty json document...
    assert_that!(parse("")).is_ok_containing(json!({}));

    // but, make sure we properly return None for an empty message
    // (empty lines should be caught by the plaintext parser)
    let config = Arc::new(Config::from_iter_safe(vec![""]).unwrap());
    let parsed = parse_logrus(config, "", None)
      .map_err(|e| SimpleError::new(format!("{:?}", e)));

    assert_that!(parsed).is_ok().is_none();
  }

  #[test]
  fn test_simple() {
    assert_that!(parse("foo=bar")).is_ok_containing(json!({
      "foo": "bar"
    }));

    assert_that!(parse(r#"foo="bar""#)).is_ok_containing(json!({
      "foo": "bar"
    }));

    // TODO: single-quoted strings aren't supported
    assert_that!(parse("foo='bar'")).is_ok_containing(json!({
      "foo": "'bar'"
    }));

    assert_that!(parse("foo=1")).is_ok_containing(json!({
      "foo": 1
    }));

    // TODO: floating-point numbers are treated as bare strings
    assert_that!(parse("foo=1.5")).is_ok_containing(json!({
      "foo": "1.5"
    }));

    assert_that!(parse("foo=&{bar}")).is_ok_containing(json!({
      "foo": "&{bar}"
    }));

    assert_that!(parse(r#"foo="hello world""#)).is_ok_containing(json!({
      "foo": "hello world"
    }));

    assert_that!(parse(r#"foo="hello 'world'""#)).is_ok_containing(json!({
      "foo": "hello 'world'"
    }));

    // TODO: need to unescape to make escaped strings reasonable
    assert_that!(parse(r#"foo="hello \"world\"""#)).is_ok_containing(json!({
      "foo": "hello \\\"world\\\""
    }));

  }

  #[test]
  fn test_invalid() {
    assert_that!(parse("foo")).is_err();
    assert_that!(parse("foo=")).is_err();
    assert_that!(parse(r#"foo=""#)).is_err();

    assert_that!(parse(r#"foo="hello "world"""#)).is_err();

    // TODO: single-quoted strings aren't supported
    //assert_that!(parse(r#"foo='bar"#)).is_err();
  }

  #[test]
  fn test_message() {
    assert_that!(parse_message(
      r#"time="2019-07-10T14:14:13.950289Z" level=debug msg="hello world""#
    )).is_ok_containing(json!({
      "kind": "logrus",
      "timestamp": "2019-07-10T14:14:13.950289Z",
      "level": "debug",
      "text": "hello world",
      "mapped_fields": {"level": "level", "msg": "text", "time": "timestamp"}
    }));
  }

  #[test]
  fn test_logrus_docs() {
    assert_that!(parse_message(concat!(
      r#"time="2015-03-26T01:27:38-04:00" "#,
      r#"level=debug "#,
      r#"msg="Started observing beach" "#,
      r#"animal=walrus number=8"#
    ))).is_ok_containing(json!({
      "kind": "logrus",
      "timestamp": "2015-03-26T05:27:38Z",
      "level": "debug",
      "text": "Started observing beach",
      "mapped_fields": {"level": "level", "msg": "text", "time": "timestamp"},
      "metadata": {
        "animal": "walrus",
        "number": 8
      }
    }));

    assert_that!(parse_message(concat!(
      r#"time="2015-03-26T01:27:38-04:00" "#,
      r#"level=fatal "#,
      r#"msg="The ice breaks!" "#,
      r#"err=&{0x2082280c0 map[animal:orca size:9009] "#,
      r#"2015-03-26 01:27:38.441574009 -0400 EDT panic It's over 9000!} "#,
      r#"number=100 omg=true"#
    ))).is_ok_containing(json!({
      "kind": "logrus",
      "timestamp": "2015-03-26T05:27:38Z",
      "level": "fatal",
      "text": "The ice breaks!",
      "mapped_fields": {"level": "level", "msg": "text", "time": "timestamp"},
      "metadata": {
        "omg": true,
        "number": 100,
        "err": "&{0x2082280c0 map[animal:orca size:9009] 2015-03-26 01:27:38.441574009 -0400 EDT panic It\'s over 9000!}"
      }
    }))
  }
}
