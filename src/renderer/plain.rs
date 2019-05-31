// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::cmp::max;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::{self, JoinHandle};

use crate::classifier::Chunk;
use crate::config::Config;
use crate::style::StyleProfile;
use crate::renderer::types::*;
use crate::renderer::common::*;

lazy_static! {
  static ref DUMMY_STYLE: StyleProfile = StyleProfile::plain();
  static ref DUMMY_CHUNK: RenderedChunk = RenderedChunk::empty();
}

fn plain_render_chunk(chunk: &Chunk) -> Vec<RenderedChunk> {
  let mut rendered_chunks = Vec::new();

  if let Some(value) = &chunk.value {
    let content = if let Some(fixed_width) = fixed_width(chunk.kind) {
      align(&value, fixed_width, chunk.alignment)
    } else {
      value.clone()
    };

    let length = content.chars().count();
    rendered_chunks.push(RenderedChunk {
      content,
      width: length,
      pad_left: chunk.pad_left,
      pad_right: chunk.pad_right,
      break_after: chunk.break_after,
      force_break_after: chunk.force_break_after,

      kind: chunk.kind,
      weight: chunk.weight,
      alignment: chunk.alignment,
    })
  }

  rendered_chunks.extend(
    chunk.children.iter()
      .flat_map(|c| plain_render_chunk(c))
  );

  rendered_chunks
}

fn plain_render_region<'a, I>(chunks: I) -> Vec<RenderedChunk>
where
  I: IntoIterator<Item = &'a Chunk>
{
  let rendered_chunks: Vec<RenderedChunk> = chunks.into_iter()
    .flat_map(|c| plain_render_chunk(c))
    .collect();

  simple_wrap_chunks(&rendered_chunks).iter()
    .map(|line_chunks| merge_chunks(line_chunks, &DUMMY_STYLE))
    .collect()
}

pub fn plain_render(entry: &MessageEntry) -> Vec<String> {
  // don't bother with the right column
  let (left, center, _) = bucketize(entry.chunks.iter());

  let left_rendered = plain_render_region(left);
  let left_width = measure_chunks(&left_rendered);
  let left_spacer = RenderedChunk::spacer(left_width, &DUMMY_STYLE);

  let center_rendered = plain_render_region(center);

  let mut ret = Vec::new();
  for i in 0..max(left_rendered.len(), center_rendered.len()) {
    let left_chunk = left_rendered.get(i).unwrap_or(&left_spacer);
    let center_chunk = center_rendered.get(i).unwrap_or(&DUMMY_CHUNK);

    ret.push(merge_chunks(
      vec![left_chunk, center_chunk],
      &DUMMY_STYLE
    ).content);
  }
  ret
}

pub fn plain_renderer(_: Arc<Config>, rx: Receiver<LogEntry>) -> JoinHandle<()> {
  thread::Builder::new().name("plain_renderer".to_string()).spawn(move || {
    for entry in rx {
      if entry.eof.is_some() {
        break;
      }

      if let Some(message) = entry.message {
        for line in plain_render(&message) {
          println!("{}", line);
        }
      }
    }
  }).unwrap()
}
