// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashSet;

use crate::parser::{Message, MessageKind};
use super::types::*;
use super::util::clean_path;

static KELOG_MAPPED_FIELDS: &[&str] = &["@timestamp", "msg", "level"];
static KELOG_METADATA_FIELDS: &[&str] = &["context"];

static CONTEXT_FIELD: &str = "context";

fn is_kelog(message: &Message) -> bool {
  if message.kind != MessageKind::Json {
    return false;
  }

  for field in KELOG_MAPPED_FIELDS {
    if !message.mapped_fields.contains_key(*field) {
      return false;
    }
  }

  for field in KELOG_METADATA_FIELDS {
    if !message.metadata.contains_key(*field) {
      return false;
    }
  }

  true
}

fn extract_context(message: &Message) -> Option<Chunk> {
  let ctx = match message.metadata.get(CONTEXT_FIELD).and_then(|c| c.as_str()) {
    Some(context) => clean_path(context),
    None => return None
  };

  Some(Chunk {
    kind: ChunkKind::Context,
    slot: ChunkSlot::Right,

    alignment: ChunkAlignment::Right,
    weight: ChunkWeight::Low.value(),

    value: Some(ctx),

    pad_left: true,
    pad_right: true,
    force_break_after: true,

    ..Default::default()
  })
}

pub fn classify_kelog(message: &Message, fields: &mut HashSet<String>) -> Vec<Chunk> {
  let mut ret = Vec::new();
  if !is_kelog(message) {
    return ret;
  }

  if let Some(context) = extract_context(message) {
    ret.push(context);
    fields.insert(CONTEXT_FIELD.to_string());
  }

  ret
}
