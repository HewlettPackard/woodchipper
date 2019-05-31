// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashMap;

use chrono::prelude::*;
use dtparse::{Parser, ParseError};

/// Convert a datetime to UTC if an offset is available
pub fn normalize_datetime(
  datetime: &NaiveDateTime, offset: Option<FixedOffset>
) -> DateTime<Utc> {
  if let Some(offset) = offset {
    if let Some(local_fixed) = offset.from_local_datetime(&datetime).earliest() {
      return Utc.from_utc_datetime(&local_fixed.naive_utc());
    }
  }

  // if we can't convert, just assume utc
  Utc.from_utc_datetime(datetime)
}

/// Leniently parses a timestamp using dtparse
pub fn parse_timestamp(timestamp: &str) -> Result<(NaiveDateTime, Option<FixedOffset>), ParseError> {
  let parser = Parser::default();

  // adapted from https://github.com/bspeice/dtparse/blob/master/src/lib.rs#L1285
  let res = parser.parse(
    timestamp,
    None, // dayfirst
    None, // yearfirst
    true, // fuzzy
    false, // fuzzy_with_tokens
    None, // default
    false, // ignoretz
    &HashMap::new() // tzinfos
  )?;

  Ok((res.0, res.1))
}


