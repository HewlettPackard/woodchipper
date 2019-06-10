// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashSet;

use crate::parser::Message;
use super::types::*;
use super::util::clean_path;

static FILE_FIELD: &str = "file";
static CALLER_FIELD: &str = "caller";

fn context_chunk(context: String) -> Chunk {
  Chunk {
    kind: ChunkKind::Context,
    slot: ChunkSlot::Right,
    alignment: ChunkAlignment::Right,
    weight: ChunkWeight::Low.value(),

    value: Some(context),

    pad_left: true,
    pad_right: true,
    force_break_after: true,

    ..Default::default()
  }
}

pub fn classify_context(
  message: &Message, fields: &mut HashSet<String>
) -> Vec<Chunk> {
  let meta = &message.metadata;

  let mut ret: Vec<Chunk> = Vec::new();

  if let Some(file) = meta.get(FILE_FIELD).and_then(|c| c.as_str()) {
    fields.insert(FILE_FIELD.to_string());

    ret.push(context_chunk(clean_path(file)));
  } else if let Some(caller) = meta.get(CALLER_FIELD).and_then(|c| c.as_str()) {
    fields.insert(CALLER_FIELD.to_string());

    ret.push(context_chunk(caller.to_string()));
  }

  ret
}
