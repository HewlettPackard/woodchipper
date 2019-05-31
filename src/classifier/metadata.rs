// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashSet;

use serde_json::Value;

use crate::parser::Message;
use super::types::*;

/// to_string with a special case for actual strings
/// by default serde stringifies them json style i.e. quotes them
/// this unpacks them to avoid printing undesired quotes
fn nicer_to_string(value: &Value) -> String {
  if let Some(val) = value.as_str() {
    String::from(val)
  } else {
    value.to_string()
  }
}

fn field_to_chunk((key, val): (&String, &Value)) -> Chunk {
  Chunk {
    kind: ChunkKind::Field,
    slot: ChunkSlot::Center,

    // fields are currently treated as part of the message text, so they
    // should always be displayed (and often messages will consist of _only_
    // fields, so pruning will result in empty lines)
    weight: ChunkWeight::High.value(),
    value: None,

    children: vec![
      Chunk {
        kind: ChunkKind::FieldKey,
        slot: ChunkSlot::Left,

        pad_left: true,

        weight: ChunkWeight::Normal.value(),
        value: Some(format!("{}=", key.to_string())),

        ..Default::default()
      },
      Chunk {
        kind: ChunkKind::FieldValue,
        slot: ChunkSlot::Left,

        pad_right: true,

        weight: ChunkWeight::Normal.value(),
        value: Some(nicer_to_string(val)),

        ..Default::default()
      }
    ],

    ..Default::default()
  }
}

pub fn classify_metadata(message: &Message, fields: &mut HashSet<String>) -> Vec<Chunk> {
  let mut fields: Vec<Chunk> = message.metadata.iter()
    .filter(|(key, _)| !fields.contains(*key))
    .filter(|(_, val)| !nicer_to_string(val).is_empty())
    .map(field_to_chunk)
    .collect();

  // todo: hoisting out measure here could save up to 5% perf
  // could also use .len() rather than .chars().count() for a rougher but faster
  // alternative
  // also consider a BinaryHeap or other sorted data structure?
  fields.sort_by_key(|c| c.measure());

  fields
}
