// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashSet;

use chrono::Local;

use crate::parser::Message;
use super::types::*;

pub fn classify_timestamp(message: &Message, _fields: &mut HashSet<String>) -> Vec<Chunk> {
  let maybe_timestamp = if let Some(timestamp) = &message.timestamp {
    Some(*timestamp)
  } else if let Some(meta) = &message.reader_metadata {
    if let Some(timestamp) = meta.timestamp {
      Some(timestamp)
    } else {
      None
    }
  } else {
    None
  };

  let formatted_date;
  let formatted_time;
  if let Some(timestamp) = maybe_timestamp {
    let local = timestamp.with_timezone(&Local);
    formatted_date = local.format("%Y-%m-%d").to_string();
    formatted_time = local.format("%H:%M:%S").to_string();
  } else {
    formatted_date = "-".to_string();
    formatted_time = "-".to_string()
  };

  vec![
    Chunk {
      kind: ChunkKind::Date,
      slot: ChunkSlot::Left,

      alignment: ChunkAlignment::Right,
      weight: ChunkWeight::Normal.value(),
      pad_right: true,

      value: Some(formatted_date),

      ..Default::default()
    },
    Chunk {
      kind: ChunkKind::Time,
      slot: ChunkSlot::Left,

      alignment: ChunkAlignment::Right,
      weight: ChunkWeight::Medium.value(),
      pad_right: true,

      value: Some(formatted_time),

      ..Default::default()
    },
  ]
}
