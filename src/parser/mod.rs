// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

pub mod util;
mod types;
mod logrus;
mod json;
mod plain;

use std::error::Error;

pub use types::{LogLevel, Message, MessageKind, ReaderMetadata, Parser};

static PARSERS: &[Parser] = &[
  json::parse_json,
  logrus::parse_logrus,
  plain::parse_plain
];

pub fn parse(
  line: &str, meta: Option<ReaderMetadata>
) -> Result<Option<Message>, Box<Error>> {
  for parser_fn in PARSERS {
    let result = parser_fn(line, meta.clone());

    match result {
      Ok(None) => continue,
      _ => return result
    };
  }

  Ok(None)
}
