// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashSet;

use crate::parser::Message;
use super::types::*;

pub fn classify_text(message: &Message, _fields: &mut HashSet<String>) -> Vec<Chunk> {
  if let Some(text) = &message.text {
    let lines: Vec<&str> = text.lines().collect();
    let mut ret = Vec::new();

    for line in lines.iter() {
      ret.push(Chunk {
        kind: ChunkKind::Text,
        slot: ChunkSlot::Center,
        weight: ChunkWeight::High.value(),

        value: Some(line.to_string()),

        pad_left: true,
        pad_right: true,
        break_after: true,
        wrap: true,

        // force a line break if there are multiple lines
        // we'll also force a break on the last line in this case: attrs
        // on the same line should start on a fresh line of their own after
        // a multi-line message
        force_break_after: lines.len() > 1,

        ..Default::default()
      })
    }

    ret
  } else {
    vec![]
  }
}

