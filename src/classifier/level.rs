// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashSet;

use crate::parser::{LogLevel, Message};
use super::types::*;

pub fn classify_level(
  message: &Message, _fields: &mut HashSet<String>
) -> Vec<Chunk> {
  let level = message.level.unwrap_or(LogLevel::Plain);
  let level_str = level.to_string().to_lowercase();

  vec![Chunk {
    kind: ChunkKind::Level(level),
    slot: ChunkSlot::Left,
    value: Some(level_str),
    weight: ChunkWeight::High.value(),

    pad_left: true,
    pad_right: true,
    break_after: true,
    alignment: ChunkAlignment::Right,

    ..Default::default()
  }]
}
