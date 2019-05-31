// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::VecDeque;
use std::cmp::max;

use textwrap::{Wrapper, NoHyphenation};

use crate::style::StyleProfile;
use crate::classifier::{
  Chunk, ChunkKind, ChunkSlot, ChunkAlignment, ChunkWeight
};
use crate::renderer::MessageEntry;

#[cfg(test)] use spectral::prelude::*;

#[derive(Debug, Clone)]
pub struct RenderedChunk {
  /// content of this chunk, potentially styled
  pub content: String,

  /// the actual width in screen characters of this span of text
  pub width: usize,

  /// if true, padding should be inserted before this span of text if preceded
  /// by another chunk (i > 0)
  pub pad_left: bool,

  /// if true, padding should be inserted after this span of text if proceeded
  pub pad_right: bool,

  /// if true, prefer to wrap elements following this one
  pub break_after: bool,

  /// if true, always wrap after this line (~= \n)
  pub force_break_after: bool,

  pub kind: ChunkKind,
  pub weight: i8,
  pub alignment: ChunkAlignment
}

impl RenderedChunk {
  pub fn empty() -> Self {
    RenderedChunk {
      content: "".to_string(),
      width: 0,
      pad_left: true,
      pad_right: true,
      break_after: false,
      force_break_after: false,
      kind: ChunkKind::Spacer,
      weight: 0,
      alignment: ChunkAlignment::Left
    }
  }

  pub fn spacer(width: usize, profile: &StyleProfile) -> Self {
    let mut space = format!("{:w$}", "", w=width);
    if profile.is_opaque() {
      space = profile.get_base().paint(space).to_string(); 
    }

    RenderedChunk {
      content: space,
      width,
      pad_left: true,
      pad_right: true,
      break_after: false,
      force_break_after: false,
      kind: ChunkKind::Spacer,
      weight: 0,
      alignment: ChunkAlignment::Left
    }
  }
}

/// merges several sub-chunks into a single chunk, baking in padding as needed
pub fn merge_chunks<'a, I>(chunks: I, profile: &StyleProfile) -> RenderedChunk
where
  I: IntoIterator<Item = &'a RenderedChunk>
{
  let mut width = 0;
  let mut pad_left = false;
  let mut buf = String::new();
  let mut last_child_pad_right = false;
  let mut last_child_break_after = false;
  let mut last_child_force_break_after = false;

  for (i, chunk) in chunks.into_iter().enumerate() {
    if chunk.width == 0 {
      continue;
    }

    // bubble up padding for first/last children
    if i == 0 && chunk.pad_left {
      pad_left = true;
    }

    // bake in padding for children, but only for middle children
    // i.e., only i > 0 and left padding
    if i > 0 && (last_child_pad_right || chunk.pad_left) {
      // ugly to shove styling this late in the process, but oh well
      if profile.is_opaque() {
        buf.push_str(&profile.get_base().paint(" ").to_string());
      } else {
        buf.push(' ');
      }
      width += 1;
    }

    buf.push_str(&chunk.content);
    width += chunk.width;

    last_child_pad_right = chunk.pad_right;
    last_child_break_after = chunk.break_after;
    last_child_force_break_after = chunk.force_break_after;
  }

  RenderedChunk {
    width, pad_left,
    content: buf,
    pad_right: last_child_pad_right,
    break_after: last_child_break_after,
    force_break_after: last_child_force_break_after,
    kind: ChunkKind::Other,
    alignment: ChunkAlignment::Left,
    weight: 0
  }
}

/// merges several chunks into a single chunk, ignoring internal padding
pub fn merge_chunks_unpadded<'a, I>(chunks: I) -> RenderedChunk
where
  I: IntoIterator<Item = &'a RenderedChunk>
{
  let mut width = 0;
  let mut buf = String::new();

  let mut pad_left = false;
  let mut last_child_pad_right = false;
  let mut last_child_break_after = false;
  let mut last_child_force_break_after = false;

  for (i, chunk) in chunks.into_iter().enumerate() {
    if chunk.width == 0 {
      continue;
    }

    if i == 0 {
      pad_left = chunk.pad_left;
    }

    buf.push_str(&chunk.content);
    width += chunk.width;

    last_child_pad_right = chunk.pad_right;
    last_child_break_after = chunk.break_after;
    last_child_force_break_after = chunk.force_break_after;
  }

  RenderedChunk {
    width, pad_left,

    content: buf,
    pad_right: last_child_pad_right,
    break_after: last_child_break_after,
    force_break_after: last_child_force_break_after,
    kind: ChunkKind::Other,
    alignment: ChunkAlignment::Left,
    weight: 0
  }
}

/// an adapted merge_chunks that cheaply measures chunks
pub fn measure_chunks<'a, I>(chunks: I) -> usize
where
  I: IntoIterator<Item = &'a RenderedChunk>
{
  let mut width = 0;
  let mut last_child_pad_right = false;

  for (i, chunk) in chunks.into_iter().enumerate() {
    if i > 0 && (last_child_pad_right || chunk.pad_left) {
      width += 1;
    }

    width += chunk.width;
    last_child_pad_right = chunk.pad_right;
  }

  width
}

pub fn largest_chunk<'a, I>(chunks: I) -> usize
where
  I: IntoIterator<Item = &'a RenderedChunk>
{
  chunks.into_iter()
    .map(|c| c.width)
    .max()
    .unwrap_or(0)
}

/// splits a chunk list into potentially several lines, each of which fits
/// within the given max_width
/// note that currently individual chunks are never split
/// if the chunks all fit in one line, the return vec will only have 1 entry
pub fn wrap_chunks<'a, I>(
  chunks: I, max_width: usize
) -> Vec<Vec<RenderedChunk>>
where
  I: IntoIterator<Item = &'a RenderedChunk>
{
  let mut collected_chunks: VecDeque<&RenderedChunk> = chunks
    .into_iter()
    .collect();

  let mut i = 0;
  let mut lines: Vec<Vec<RenderedChunk>> = Vec::new();
  let mut current_line: Vec<RenderedChunk> = Vec::new();
  let mut current_line_will_wrap = 
    measure_chunks(collected_chunks.iter().cloned()) > max_width;
  let mut line_width = 0;
  let mut last_pad_right = false;
  let mut last_break_after = false;
  let mut last_force_break_after = false;

  while let Some(chunk) = collected_chunks.pop_front() {
    // how much space will this chunk take if we add it to the current line?
    let padded_width = if i > 0 && (last_pad_right || chunk.pad_left) {
      1
    } else {
      0
    } + chunk.width;

    // if it won't fit, start a new line
    let wrap_length = line_width + padded_width > max_width;

    // if the previous element has `break_after` set *and* we expect to wrap
    // this line, wrap now (potentially several chunks early)
    let wrap_early = current_line_will_wrap && last_break_after;

    // if there's no padding between this chunk and the next chunk, also wrap
    // early (but at most 1 chunk early)
    // e.g. given a metadata chunk like [foo=][bar], don't wrap in the unpadded
    // space between the two chunks
    let next_chunk = collected_chunks.get(0);
    let wrap_slightly_early = if let Some(next_chunk) = next_chunk {
      let next_chunk_is_attached = !chunk.pad_right && !next_chunk.pad_left;
      let next_chunk_will_overflow =
        line_width + padded_width + next_chunk.width > max_width;

      next_chunk_is_attached && next_chunk_will_overflow
    } else {
      false
    };

    let should_wrap = wrap_length
      || wrap_early
      || wrap_slightly_early
      || last_force_break_after;

    // if this is a fresh line, we need to fit the chunk in regardless of
    // whether or not it will actually fit
    if line_width > 0 && should_wrap {
      lines.push(current_line);

      line_width = 0;
      current_line = Vec::new();
      current_line_will_wrap =
        measure_chunks(collected_chunks.iter().cloned()) > max_width;
      last_pad_right = false;
      last_break_after = false;
    } else {
      last_pad_right = chunk.pad_right;
      last_break_after = chunk.break_after;
    }

    last_force_break_after = chunk.force_break_after;

    line_width += padded_width;
    current_line.push(chunk.clone());

    i += 1;
  }

  lines.push(current_line);

  lines
}

/// a simpler wrapping function that only accounts for newlines within a message
/// (i.e. Chunk.force_break_after is true)
pub fn simple_wrap_chunks<'a, I>(chunks: I) -> Vec<Vec<RenderedChunk>> 
where
  I: IntoIterator<Item = &'a RenderedChunk>
{
  let mut lines: Vec<Vec<RenderedChunk>> = Vec::new();
  let mut current_line: Vec<RenderedChunk> = Vec::new();
  let mut last_force_break_after = false;

  for chunk in chunks.into_iter() {
    if last_force_break_after {
      lines.push(current_line);
      current_line = Vec::new();
    }

    last_force_break_after = chunk.force_break_after;
    current_line.push(chunk.clone());
  }

  lines.push(current_line);

  lines
}

/// left pads a chunk with a trailing spacer to ensure it is exactly
/// `max_width` columns long
pub fn left_pad_chunk(
  chunk: &RenderedChunk, max_width: usize, profile: &StyleProfile
) -> RenderedChunk {
  if let Some(remaining) = max_width.checked_sub(chunk.width) {
    merge_chunks_unpadded(
      vec![chunk, &RenderedChunk::spacer(remaining, profile)]
    )
  } else {
    chunk.clone()
  }
}

/// right pads a chunk with a trailing spacer to ensure it is exactly
/// `max_width` columns long
pub fn right_pad_chunk(
  chunk: &RenderedChunk, max_width: usize, profile: &StyleProfile
) -> RenderedChunk {
  if let Some(remaining) = max_width.checked_sub(chunk.width) {
    merge_chunks_unpadded(
      vec![&RenderedChunk::spacer(remaining, profile), chunk]
    )
  } else {
    chunk.clone()
  }
}

pub fn fixed_width(kind: ChunkKind) -> Option<usize> {
  match kind {
    ChunkKind::Date => Some(10),
    ChunkKind::Time => Some(8),
    ChunkKind::Level(_) => Some(7),
    _ => None
  }
}

pub fn align(content: &str, width: usize, alignment: ChunkAlignment) -> String {
  match alignment {
    ChunkAlignment::Left => format!("{:<width$}", content, width=width),
    ChunkAlignment::Right => format!("{:>width$}", content, width=width)
  }
}

/// organizes a list of chunks into buckets by their slot
pub fn bucketize<'a, I>(chunks: I)
-> (Vec<&'a Chunk>, Vec<&'a Chunk>, Vec<&'a Chunk>)
where
  I: IntoIterator<Item = &'a Chunk>
{
  let mut left = Vec::new();
  let mut center = Vec::new();
  let mut right = Vec::new();

  for chunk in chunks.into_iter() {
    match chunk.slot {
      ChunkSlot::Left => left.push(chunk),
      ChunkSlot::Center => center.push(chunk),
      ChunkSlot::Right => right.push(chunk)
    };
  }

  (left, center, right)
}

/// renders a single chunk into one or more RenderedChunk
/// these chunks are semantically intended to appear on one line, but may be
/// wrapped later if necessary
/// note that `wrap_width` here should be the width of only the current screen
/// region (e.g. left/right/center) and is used to split long chunks into one
/// or more RenderedChunks, in addition to any child chunks they may contain
fn styled_render_chunk(
  chunk: &Chunk,
  profile: &StyleProfile, wrap_width: Option<usize>
) -> Vec<RenderedChunk> {
  let chunk_style = profile.get_style(&chunk.kind);

  let mut rendered_chunks = Vec::new();
  
  if let Some(value) = &chunk.value {
    // todo: would like to use iters here but apparently that needs sorcery
    let wrapped: Vec<String> = match wrap_width.filter(|_| chunk.wrap) {
      Some(wrap_width) => {
        let wrapper = Wrapper::with_splitter(wrap_width, NoHyphenation);

        wrapper.wrap_iter(value).map(|v| v.to_string()).collect()
      },
      // sad clone :(
      None => vec![value.clone()]
    };

    for wrapped_line in wrapped {
      // TODO: decide if we should apply fixed width to all wrapped lines
      let content = if let Some(fixed_width) = fixed_width(chunk.kind) {
        align(&wrapped_line, fixed_width, chunk.alignment)
      } else {
        wrapped_line
      };

      let length = content.chars().count();
      rendered_chunks.push(RenderedChunk {
        content: chunk_style.paint(content).to_string(),
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
  }

  rendered_chunks.extend(
    chunk.children.iter()
      .flat_map(|c| styled_render_chunk(c, profile, wrap_width))
  );

  rendered_chunks
}

/// renders a subset of chunks into wrapped lines
/// rendered chunks are merged such that each returned RenderedChunk can be
/// displayed on its own line (possibly with additional chunks on the side)
fn styled_render_region(
  chunks: Vec<&Chunk>,
  profile: &StyleProfile,
  wrap_width: Option<usize>
) -> Vec<RenderedChunk> {
  let rendered_chunks: Vec<RenderedChunk> = chunks.iter()
    .flat_map(|c| styled_render_chunk(c, profile, wrap_width))
    .collect();

  if let Some(wrap_width) = wrap_width {
    wrap_chunks(&rendered_chunks, wrap_width).iter()
      .map(|line_chunks| merge_chunks(line_chunks, &profile))
      .collect()
  } else {
    simple_wrap_chunks(&rendered_chunks).iter()
      .map(|line_chunks| merge_chunks(line_chunks, &profile))
      .collect()
  }
}

/// filters chunks for only those with weight >= min
fn prune(chunks: Vec<&Chunk>, min: i8) -> Vec<&Chunk> {
  chunks.into_iter().filter(|c| c.weight >= min).collect()
}

fn prune_level(wrap_width: Option<usize>) -> ChunkWeight {
  if let Some(width) = wrap_width {
    if width < 60 {
      return ChunkWeight::High;
    } else if width < 80 {
      return ChunkWeight::Medium;
    } else if width < 100 {
      return ChunkWeight::Normal;
    }
  }

  ChunkWeight::Low
}

/// renders a MessageEntry into a list of strings wrapped to fit `width`
pub fn styled_render(
  entry: &MessageEntry, profile: &StyleProfile, wrap_width: Option<usize>
) -> Vec<String> {
  // TODO: if wrapping is disabled, use measure_chunks before splitting
  // into buckets to prune fields based on weight
  // for now, just skip rendering the right column if wrapping is disabled
  // TODO: allow left and right columns to wrap as well?
  let min_weight = prune_level(wrap_width).value();

  let (left, center, right) = bucketize(entry.chunks.iter());
  let right_is_empty = right.is_empty();
  let left_rendered = styled_render_region(
    prune(left, min_weight), profile, None
  );
  let left_width = largest_chunk(&left_rendered);
  let right_rendered = styled_render_region(
    prune(right, min_weight), profile, None
  );
  let right_width = largest_chunk(&right_rendered);
  
  let center_width = match wrap_width {
    Some(wrap_width) => 
      if right_is_empty || left_width + right_width + 2 > wrap_width {
        // not enough room for the right side
        // TODO: this can still overflow and panic for really tiny widths
        wrap_width - left_width - 1
      } else {
        // we can render all 3 columns
        wrap_width - left_width - right_width - 2
      },
    
    // don't render the right column if wrapping is disabled
    // TODO: reevaluate this in the future
    None => 0
  };

  let center_rendered = styled_render_region(
    prune(center, min_weight), profile, Some(center_width)
  );

  let left_spacer = RenderedChunk::spacer(left_width, profile);
  let center_spacer = RenderedChunk::spacer(center_width, profile);
  let right_spacer = RenderedChunk::spacer(right_width, profile);

  let mut ret = Vec::new();

  let max_height = max(
    left_rendered.len(),
    max(center_rendered.len(), right_rendered.len())
  );
  for i in 0..max_height {
    let left_chunk = left_rendered.get(i).unwrap_or(&left_spacer);
    let center_chunk = left_pad_chunk(
      center_rendered.get(i).unwrap_or(&center_spacer),
      center_width,
      profile
    );

    let right_chunk = right_pad_chunk(
      right_rendered.get(i).unwrap_or(&right_spacer),
      right_width,
      profile
    );

    ret.push(merge_chunks(
      vec![left_chunk, &center_chunk, &right_chunk],
      profile
    ).content);
  }

  ret
}

#[cfg(test)]
mod tests {
  use super::*;

  fn spacers(count: usize) -> Vec<RenderedChunk> {
    let normal = StyleProfile::default_normal();

    (0..count)
      .map(|_| RenderedChunk::spacer(10, &normal))
      .collect()
  }

  fn get_simple_padded() -> Vec<RenderedChunk> {
    vec![
      RenderedChunk {
        content: "foo".to_string(),
        width: 3, weight: 0,
        pad_left: true, pad_right: true, break_after: false,
        force_break_after: false,
        kind: ChunkKind::Other, alignment: ChunkAlignment::Left
      },
      RenderedChunk {
        content: "bar".to_string(),
        width: 3, weight: 0,
        pad_left: true, pad_right: true, break_after: false,
        force_break_after: false,
        kind: ChunkKind::Other, alignment: ChunkAlignment::Left
      },
      RenderedChunk {
        content: "baz".to_string(),
        width: 3, weight: 0,
        pad_left: true, pad_right: true, break_after: true,
        force_break_after: false,
        kind: ChunkKind::Other, alignment: ChunkAlignment::Left
      }
    ]
  }

  fn get_simple_unpadded() -> Vec<RenderedChunk> {
    vec![
      RenderedChunk {
        content: "foo".to_string(),
        width: 3, weight: 0,
        pad_left: false, pad_right: false, break_after: false,
        force_break_after: false,
        kind: ChunkKind::Other, alignment: ChunkAlignment::Left
      },
      RenderedChunk {
        content: "bar".to_string(),
        width: 3, weight: 0,
        pad_left: false, pad_right: false, break_after: false,
        force_break_after: false,
        kind: ChunkKind::Other, alignment: ChunkAlignment::Left
      },
      RenderedChunk {
        content: "baz".to_string(),
        width: 3, weight: 0,
        pad_left: false, pad_right: false, break_after: false,
        force_break_after: false,
        kind: ChunkKind::Other, alignment: ChunkAlignment::Left
      }
    ]
  }

  fn get_field_chunk(key: &str, val: &str) -> Chunk {
    Chunk {
      kind: ChunkKind::Field,
      slot: ChunkSlot::Center,
      weight: 0,
      value: None,
      children: vec![
        Chunk {
          kind: ChunkKind::FieldKey,
          slot: ChunkSlot::Left,
          pad_left: true,
          value: Some(key.to_string()),

          ..Default::default()
        },
        Chunk {
          kind: ChunkKind::FieldValue,
          slot: ChunkSlot::Left,
          pad_right: true,
          weight: 0,
          value: Some(val.to_string()),

          ..Default::default()
        }
      ],

      ..Default::default()
    }
  }

  fn get_text_chunk(text: &str) -> Chunk {
    Chunk {
      kind: ChunkKind::Text,
      slot: ChunkSlot::Center,
      weight: 10,

      value: Some(text.to_string()),

      pad_left: true,
      pad_right: true,
      break_after: true,
      wrap: true,

      ..Default::default()
    }
  }

  fn get_tags(profile: &StyleProfile) -> Vec<RenderedChunk> {
    vec![
      styled_render_chunk(&get_field_chunk("foo=", "1"), &profile, None),
      styled_render_chunk(&get_field_chunk("bar=", "2"), &profile, None),
      styled_render_chunk(&get_field_chunk("baz=", "3"), &profile, None)
    ].into_iter().flatten().collect()
  }

  fn get_message(profile: &StyleProfile) -> Vec<RenderedChunk> {
    vec![
      styled_render_chunk(&get_text_chunk("hello world"), &profile, None),
      get_tags(&profile)
    ].into_iter().flatten().collect()
  }

  #[test]
  fn test_merge_chunks_spacer() {
    let normal = StyleProfile::default_normal();
    assert_that!(&merge_chunks(&spacers(1), &normal).width).is_equal_to(10);

    let merged = merge_chunks(&spacers(3), &normal);
    assert_that!(merged.width).is_equal_to(30);
    assert_that!(merged.content.chars().count()).is_equal_to(30);
  }

  #[test]
  fn test_merge_chunks_simple_padded() {
    let normal = StyleProfile::default_normal();
    let merged = merge_chunks(&get_simple_padded(), &normal);
    assert_that!(merged.content).is_equal_to("foo bar baz".to_string());
    assert_that!(merged.width).is_equal_to(11);
    assert_that!(merged.pad_left).is_equal_to(true);
    assert_that!(merged.pad_right).is_equal_to(true);
    assert_that!(merged.break_after).is_equal_to(true);
  }

  #[test]
  fn test_merge_chunks_simple_unpadded() {
    let normal = StyleProfile::default_normal();
    let merged = merge_chunks(&get_simple_unpadded(), &normal);
    assert_that!(merged.content).is_equal_to("foobarbaz".to_string());
    assert_that!(merged.width).is_equal_to(9);
    assert_that!(merged.pad_left).is_equal_to(false);
    assert_that!(merged.pad_right).is_equal_to(false);
    assert_that!(merged.break_after).is_equal_to(false);
  }

  #[test]
  fn test_merge_chunks_unpadded() {
    let merged = merge_chunks_unpadded(&spacers(3));
    assert_that!(merged.width).is_equal_to(30);
    assert_that!(merged.pad_left).is_equal_to(false);
    assert_that!(merged.pad_right).is_equal_to(false);

    let merged = merge_chunks_unpadded(&get_simple_padded());
    assert_that!(merged.content).is_equal_to("foobarbaz".to_string());
    assert_that!(merged.width).is_equal_to(9);
    assert_that!(merged.pad_left).is_equal_to(true);
    assert_that!(merged.pad_right).is_equal_to(true);
    assert_that!(merged.break_after).is_equal_to(true);
  }

  #[test]
  fn test_merge_chunks_tags() {
    let normal = StyleProfile::default_normal();
    let merged = merge_chunks(&get_tags(&normal), &normal);
    assert_that!(merged.width).is_equal_to(17);
    assert_that!(merged.pad_left).is_equal_to(true);
    assert_that!(merged.pad_right).is_equal_to(true);
    assert_that!(merged.break_after).is_equal_to(false);
  }

  #[test]
  fn test_merge_chunks_tags_selected() {
    let selected = StyleProfile::default_selected();
    let merged = merge_chunks(&get_tags(&selected), &selected);
    assert_that!(merged.width).is_equal_to(17);
    assert_that!(merged.pad_left).is_equal_to(true);
    assert_that!(merged.pad_right).is_equal_to(true);
    assert_that!(merged.break_after).is_equal_to(false);
  }

  #[test]
  fn test_merge_chunks_message() {
    let normal = StyleProfile::default_normal();
    let merged = merge_chunks(&get_message(&normal), &normal);
    assert_that!(merged.width).is_equal_to(29);
    assert_that!(merged.pad_left).is_equal_to(true);
    assert_that!(merged.pad_right).is_equal_to(true);
    assert_that!(merged.break_after).is_equal_to(false);
  }

  #[test]
  fn test_merge_chunks_message_selected() {
    let selected = StyleProfile::default_selected();
    let merged = merge_chunks(&get_message(&selected), &selected);
    assert_that!(merged.width).is_equal_to(29);
    assert_that!(merged.pad_left).is_equal_to(true);
    assert_that!(merged.pad_right).is_equal_to(true);
    assert_that!(merged.break_after).is_equal_to(false);
  }

  #[test]
  fn test_measure_chunks() {
    assert_that!(measure_chunks(&spacers(1))).is_equal_to(10);
    assert_that!(measure_chunks(&spacers(3))).is_equal_to(30);
    assert_that!(measure_chunks(&get_simple_padded())).is_equal_to(11);
    assert_that!(measure_chunks(&get_simple_unpadded())).is_equal_to(9);

    let normal = StyleProfile::default_normal();
    let selected = StyleProfile::default_selected();
    assert_that!(measure_chunks(&get_tags(&normal))).is_equal_to(17);
    assert_that!(measure_chunks(&get_tags(&selected))).is_equal_to(17);
    assert_that!(measure_chunks(&get_message(&normal))).is_equal_to(29);
    assert_that!(measure_chunks(&get_message(&selected))).is_equal_to(29);
  }
}
