// (C) Copyright 2020 Hewlett Packard Enterprise Development LP

use std::collections::BinaryHeap;
use std::cmp::{Ord, Ordering, PartialEq, PartialOrd};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::Utc;
use simple_error::SimpleResult;

use crate::config::Config;
use crate::parser::MessageKind;
use crate::renderer::{LogEntry, MessageEntry};

/// The default length of time messages should be held in the buffer
const DEFAULT_BUFFER_MS: u64 = 1000;

/// A wrapped struct since we need an extra timestamp
struct TimestampedEntry {
  /// Monotonic instant that this message was received from the underlying
  /// reader, used to evict messages that have been held for too long
  received: Instant,

  /// UTC timestamp as determined by the underlying reader, potentially from
  /// message metadata
  ///
  /// This should be the desired sort order where possible
  timestamp: i64,

  /// The wrapped MessageEntry
  entry: MessageEntry
}

impl TimestampedEntry {
  fn new(entry: MessageEntry) -> Self {
    // fall back to the system timestamp if none exists
    let timestamp = if let Some(timestamp) = &entry.message.timestamp {
      timestamp.timestamp_millis()
    } else if let Some(meta) = &entry.message.reader_metadata {
      if let Some(timestamp) = &meta.timestamp {
        timestamp.timestamp_millis()
      } else {
        Utc::now().timestamp_millis()
      }
    } else {
      Utc::now().timestamp_millis()
    };

    TimestampedEntry {
      received: Instant::now(),
      timestamp,
      entry
    }
  }
}

impl Ord for TimestampedEntry {
  fn cmp(&self, other: &Self) -> Ordering {
    // note: intentionally inverted (we want a min-heap)
    other.timestamp.cmp(&self.timestamp)
  }
}

impl PartialOrd for TimestampedEntry {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    other.timestamp.partial_cmp(&self.timestamp)
  }
}

impl PartialEq for TimestampedEntry {
  fn eq(&self, other: &Self) -> bool {
    self.timestamp == other.timestamp
  }
}

impl Eq for TimestampedEntry {}

/// A wrapping reader that attempts to reorder incoming message from another
/// reader such that timestamps stay (more) sequential.
pub fn read_ordered(
  config: Arc<Config>,
  rx: Receiver<LogEntry>,
  tx: Sender<LogEntry>,
) -> JoinHandle<SimpleResult<()>> {
  thread::Builder::new().name("read_ordered".to_string()).spawn(move || {
    let buffer_duration = Duration::from_millis(
      config.buffer_ms.unwrap_or(DEFAULT_BUFFER_MS)
    );

    tx.send(LogEntry::internal(&format!(
      "note: attempting to reorder messages, buffer: {}ms",
      buffer_duration.as_millis()
    ))).ok();

    // TODO: we could probably async-ify this and remove the need for sleep(100)
    // This could create issues with multiple runtimes for e.g. the kubernetes
    // reader, though.
    let mut heap: BinaryHeap<TimestampedEntry> = BinaryHeap::new();

    'outer: loop {
      thread::sleep(Duration::from_millis(100));

      // first, drain all incoming messages into the heap
      for unbuffered_entry in rx.try_iter() {
        if let Some(message) = unbuffered_entry.message {
          // immediately pass through internal messages
          if let MessageKind::Internal = message.message.kind {
            tx.send(LogEntry {
              message: Some(message),
              eof: None
            }).ok();
          } else {
            heap.push(TimestampedEntry::new(message));
          }

        } else if let Some(_) = unbuffered_entry.eof {
          // quit and send immediately (buffered messages will be discarded)
          tx.send(LogEntry::eof()).ok();
          break 'outer;
        }
      }

      // now, drain (in order) all messages from the top of the heap that pass
      // the eviction deadline
      let current_time = Instant::now();
      while let Some(entry) = heap.peek() {
        if current_time.duration_since(entry.received) >= buffer_duration {
          // if this unwrap fails, we have bigger problems...
          let real_entry = heap.pop().unwrap().entry;

          tx.send(LogEntry {
            message: Some(real_entry),
            eof: None
          }).ok();
        } else {
          break;
        }
      }
    }

    Ok(())
  }).unwrap()
}
