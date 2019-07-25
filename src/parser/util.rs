// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use chrono::prelude::*;

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
