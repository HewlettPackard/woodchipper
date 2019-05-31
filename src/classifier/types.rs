// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashSet;
use std::fmt;

use crate::parser::{LogLevel, Message};

/// A ChunkKind is a loose category for types of chunks
/// These may affect filtering and various rendering options (e.g. style,
/// alignment, etc)
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum ChunkKind {
  Level(LogLevel),

  Date,
  Time,
  Text,
  Context,

  /// A chunk containing an arbitrary key/value pair
  Field,

  /// A Field child containing a key
  FieldKey,

  /// A Field child containing a value
  FieldValue,

  Spacer,

  Other
}

impl fmt::Display for ChunkKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

/// Region of the display this chunk should be placed within
#[derive(Debug, Copy, Clone)]
pub enum ChunkSlot {
  Left,
  Center,
  Right
}

/// Text alignment for chunk content within a column
#[derive(Debug, Copy, Clone)]
pub enum ChunkAlignment {
  Left,
  Right
}

/// Chunk weight controls both on-screen ordering and rendering priority
/// Lower-priority chunks (<= 0) may be hidden if they won't fit on screen
#[derive(Debug, Copy, Clone)]
pub enum ChunkWeight {
  Low,
  Normal,
  Medium,
  High
}

impl ChunkWeight {
  pub fn value(self) -> i8 {
    match self {
      ChunkWeight::Low => -10,
      ChunkWeight::Normal => 0,
      ChunkWeight::Medium => 10,
      ChunkWeight::High => 20
    }
  }
}

#[derive(Debug, Clone)]
pub struct Chunk {
  pub kind: ChunkKind,
  pub slot: ChunkSlot,
  pub alignment: ChunkAlignment,

  pub pad_left: bool,
  pub pad_right: bool,
  pub break_after: bool,
  pub force_break_after: bool,
  pub wrap: bool,

  pub weight: i8,
  pub value: Option<String>,

  pub children: Vec<Chunk>
}

impl Chunk {
  pub fn measure(&self) -> usize {
    let self_len: usize = if let Some(value) = &self.value {
      value.chars().count()
    } else {
      0
    };

    let mut self_padding = 0;
    if self.pad_left {
      self_padding += 1;
    }

    if self.pad_right {
      self_padding += 1;
    }

    let mut child_len: usize = 0;
    for child in &self.children {
      child_len += child.measure();

      // todo: attempt to merge left/right padding if they group together
      if child.pad_left {
        child_len += 1;
      }

      if child.pad_right {
        child_len += 1;
      }
    }

    self_len + self_padding + child_len
  }
}

impl Default for Chunk {
  fn default() -> Self {
    Chunk {
      kind: ChunkKind::Other,
      slot: ChunkSlot::Left,

      alignment: ChunkAlignment::Left,
      pad_left: false,
      pad_right: false,
      break_after: false,
      force_break_after: false,
      wrap: false,

      weight: ChunkWeight::Normal.value(),
      value: None,

      children: Vec::new()
    }
  }
}

/// Given some Message, a classifier generates chunks for display
pub type Classifier = fn(message: &Message, consumed_fields: &mut HashSet<String>) -> Vec<Chunk>;
