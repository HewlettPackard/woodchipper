// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

mod types;
mod timestamp;
mod level;
mod text;
mod logrus;
mod kelog;
mod metadata;
mod source;
mod util;

use std::collections::HashSet;

pub use types::*;
use crate::parser::Message;

static CLASSIFIERS: &[Classifier] = &[
  timestamp::classify_timestamp,
  level::classify_level,
  source::classify_source,
  text::classify_text,
  logrus::classify_logrus,
  kelog::classify_kelog,
  metadata::classify_metadata
];

pub fn classify(message: &Message) -> Vec<Chunk> {
  let mut consumed_fields: HashSet<String> = HashSet::new();

  CLASSIFIERS.iter()
    .flat_map(|c| c(message, &mut consumed_fields))
    .collect()
}
