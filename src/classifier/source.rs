// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashSet;

use crate::parser::Message;
use super::types::*;

pub fn classify_source(
  message: &Message, _fields: &mut HashSet<String>
) -> Vec<Chunk> {
  if let Some(meta) = &message.reader_metadata {
    if let Some(source) = &meta.source {
      return vec![Chunk {
        kind: ChunkKind::Context,
        slot: ChunkSlot::Right,
        value: Some(source.to_string()),
        weight: ChunkWeight::Normal.value(),

        pad_left: true,
        pad_right: true,
        alignment: ChunkAlignment::Right,
        force_break_after: true,

        ..Default::default()
      }]
    }
  }

  vec![]
}
