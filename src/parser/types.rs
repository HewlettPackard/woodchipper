// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::Hash;
use std::str::FromStr;
use std::sync::Arc;

use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::config::Config;

/// Attempts to parse a log line.
/// Returns Ok(Some(Message)) on success, Err on error, or Ok(None) to pass the
/// message to the next parser.
pub type Parser = fn(
  config: Arc<Config>, line: &str, meta: Option<ReaderMetadata>
) -> Result<Option<Message>, Box<dyn Error>>;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
  Json,
  Plain,
  Logrus,
  Klog,
  Regex,
  Internal
}

impl fmt::Display for MessageKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
  Debug,
  Info,
  Warning,
  Error,
  Fatal,
  Plain,
  Int
}

impl fmt::Display for LogLevel {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

impl FromStr for LogLevel {
  type Err = ();

  fn from_str(s: &str) -> Result<LogLevel, ()> {
    match s.to_lowercase().as_str() {
      "debug" | "dbg" | "d" => Ok(LogLevel::Debug),
      "info" | "i" => Ok(LogLevel::Info),
      "warning" | "warn" | "w" => Ok(LogLevel::Warning),
      "error" | "err" | "e" => Ok(LogLevel::Error),
      "fatal" | "panic" | "f" | "p" => Ok(LogLevel::Fatal),
      _ => Err(())
    }
  }
}

/// Target fields in Message into which parsers may map input fields
/// Classifiers may use this to help determine the logging system
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MappingField {
  Timestamp,
  Level,
  Text
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReaderMetadata {
  // an external timestamp, may be overridden by message content
  pub timestamp: Option<DateTime<Utc>>,

  // message source if following multiple inputs
  pub source: Option<String>
}

fn is_empty<K: Hash + Eq, V>(map: &HashMap<K, V>) -> bool {
  map.is_empty()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
  /// The detected type of this message
  pub kind: MessageKind,

  /// The best-guess timestamp, normalized to UTC where possible
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timestamp: Option<DateTime<Utc>>,

  /// The detected log level
  #[serde(skip_serializing_if = "Option::is_none")]
  pub level: Option<LogLevel>,

  // The raw message content
  pub raw: String,

  /// The message text
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,

  /// Additional fields e.g. in json messages
  #[serde(skip_serializing_if = "is_empty")]
  pub metadata: HashMap<String, Value>,

  /// Metadata from readers (filename, k8s pod, external timestamp, etc)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub reader_metadata: Option<ReaderMetadata>,

  /// Mappings of original field names to their destination fields
  #[serde(skip_serializing_if = "is_empty")]
  pub mapped_fields: HashMap<String, MappingField>
}
