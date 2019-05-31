// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::cell::RefCell;
use std::cmp::{min, max};
use std::collections::BTreeMap;
use std::error::Error;
use std::rc::Rc;

use crossterm::{Terminal, TerminalCursor, ClearType};

use crate::renderer::types::*;
use crate::renderer::common::*;
use crate::style::StyleProfile;
use crate::renderer::interactive::state::{RenderState, RcState};

/// renders a message without displaying and returns its height
/// this is mildly expensive and should be called sparingly
fn measure_entry(state: RcState, abs_index: usize) -> usize {
  let entry = &state.entries.borrow()[abs_index];
  styled_render(
    &entry,
    &state.config.style.normal,
    Some(state.width as usize)
  ).len()
}

fn profile_for_message<'a>(
  state: &'a RenderState, message: &MessageEntry, selected: bool
) -> &'a  StyleProfile {
  if selected {
    return &state.config.style.selected;
  }

  // TODO: also highlight messages during find

  // if the user is writing a filter, highlight matching messages
  if let Some(filter) = &state.highlight_filter {
    if filter.filter(&message.message) {
      return &state.config.style.highlighted;
    }
  }

  &state.config.style.normal
}

#[derive(Debug, Clone, Copy)]
pub struct Anchor {
  /// the y offset from the bottom row at which we should anchor our rendering
  /// this should allow users to resize the window without their selection
  /// moving off-screen
  offset: u16,

  /// the height of the anchored message when the selection was created
  /// if the terminal is resized and the message is now a different height, this
  /// can be used to properly realign text
  /// note that the height may not be known when the selection is created; it
  /// will be updated on the next rendering pass
  height: Option<u16>
}

/// Represents a message selection
///
/// Normally we're anchored to the bottom and always render the latest messages.
/// If the user highlights a particular message, we should instead anchor to
/// that to prevent the viewport from moving
#[derive(Debug, Clone, Copy)]
pub struct Selection {
  /// the index within the entry list that is currently highlighted (of filtered
  /// entries)
  pub rel_index: usize,

  anchor: Anchor
}

#[derive(Clone)]
pub struct LogState {
  /// the index of the first entry at least partially displayed, inclusive
  pub range_min: usize,

  /// the index of the last entry at least partially displayed, inclusive
  pub range_max: usize,

  /// map of displayed entry rel_index -> current count of columns from the
  /// bottom
  anchors: Rc<RefCell<BTreeMap<usize, Anchor>>>,

  pub selection: Option<Selection>,
}

impl LogState {
  pub fn new() -> Self {
    LogState {
      range_min: 0,
      range_max: 0,
      anchors: Rc::new(RefCell::new(BTreeMap::new())),
      selection: None
    }
  }
}

fn render_int(
  state_mut: &mut RenderState, terminal: &Terminal, cursor: &TerminalCursor
) -> Result<(), Box<Error>> {
  // TODO: handle weak refs better, we're just blindly unwrapping right now
  // (at the moment they should never dealloc but eventually some max log size
  // should be implemented)

  // design note re: clearing: we want to reduce (as much as possible) the delay
  // between line clearing and writing content back to the screen
  // in some cases, the screen may flicker if content isn't written before the
  // terminal re-renders blank text, which is why each component is responsible
  // for its own clearing rather than just clearing the entire screen at the
  // beginning of each render

  let mut anchors = state_mut.log.anchors.borrow_mut();
  let filtered_entries = state_mut.filtered_entries.borrow();

  anchors.clear();
  if filtered_entries.is_empty() || state_mut.height < 2 {
    state_mut.log.range_min = 0;
    state_mut.log.range_max = 0;
    terminal.clear(ClearType::All)?;
    return Ok(());
  }

  let start_selected: bool;
  let start_index: usize;
  let mut start_y: u16;
  let start_height;
  let end_y = state_mut.height - 1; // last valid y pos (inclusive)

  if let Some(selection) = state_mut.log.selection {
    start_selected = true;
    start_index = selection.rel_index;
    start_y = match end_y.checked_sub(selection.anchor.offset) {
      Some(offset) => offset,
      None => end_y
    } as u16;
    start_height = selection.anchor.height;
  } else {
    start_selected = false;
    start_index = filtered_entries.len() - 1;
    start_y = end_y; // we'll adjust for longer entries shortly
    start_height = None;
  }

  state_mut.log.range_max = start_index;
  state_mut.log.range_min = start_index;

  // render the anchored entry first so we can decide if start_y is still
  // valid
  let start_entry = &filtered_entries[start_index].entry.upgrade().unwrap();
  let start_lines = styled_render(
    start_entry,
    profile_for_message(&state_mut, start_entry, start_selected),
    Some(state_mut.width as usize)
  );

  // if the message height has changed (e.g. due to a resize),
  // update the position to keep it anchored
  if let Some(start_height) = start_height {
    let diff = start_height as isize - start_lines.len() as isize;

    if diff != 0 {
      start_y = min(
        max(start_y as isize + diff, 0),
        state_mut.height as isize - 1
      ) as u16;

      if let Some(old_selection) = state_mut.log.selection {
        state_mut.log.selection = Some(Selection {
          rel_index: old_selection.rel_index,
          anchor: Anchor {
            offset: end_y - start_y,
            height: Some(start_lines.len() as u16)
          }
        });
      }
    }
  }

  // if the entry won't fit, it may be too long or the term was resized
  // either way, we'll need to adjust the anchor to make room
  if start_y as usize + start_lines.len() > end_y as usize {
    start_y = max(end_y as isize - start_lines.len() as isize, 0) as u16;

    // also update the selection if necessary
    if let Some(old_selection) = state_mut.log.selection {
      state_mut.log.selection = Some(Selection {
        rel_index: old_selection.rel_index,
        anchor: Anchor {
          offset: end_y - start_y,
          height: Some(start_lines.len() as u16)
        }
      });
    }
  }

  let mut y_pos = start_y;

  anchors.insert(start_index, Anchor {
    offset: end_y - start_y,
    height: Some(start_lines.len() as u16)
  });

  // actually render that first entry (or as much of it as possible)
  for line in start_lines {
    cursor.goto(0, y_pos as u16)?;
    terminal.clear(ClearType::CurrentLine)?;
    terminal.write(line)?;

    y_pos += 1;
    if y_pos >= end_y {
      break;
    }
  }

  // now render as many entries below it as possible
  if y_pos < end_y {
    'outer_down: for i in {start_index + 1 .. filtered_entries.len()} {
      let entry = &filtered_entries[i].entry.upgrade().unwrap();
      let lines = styled_render(
        entry,
        profile_for_message(&state_mut, entry, false),
        Some(state_mut.width as usize)
      );

      state_mut.log.range_max = i;
      anchors.insert(i, Anchor {
        offset: end_y - y_pos,
        height: Some(lines.len() as u16)
      });

      for line in lines {
        cursor.goto(0, y_pos)?;
        terminal.clear(ClearType::CurrentLine)?;
        terminal.write(line)?;

        y_pos += 1;
        if y_pos >= end_y {
          break 'outer_down;
        }
      }
    }
  }

  // clear any space at the bottom (unlikely, but possible)
  if y_pos < end_y {
    cursor.goto(0, y_pos)?;
    terminal.clear(ClearType::FromCursorDown)?;
  }

  // now reset y_pos and render upward
  if start_y > 0 && start_index > 0 {
    y_pos = start_y - 1;

    'outer_up: for i in {0..start_index}.rev() {
      let entry = &filtered_entries[i].entry.upgrade().unwrap();
      let lines = styled_render(
        entry,
        profile_for_message(&state_mut, entry, false),
        Some(state_mut.width as usize)
      );
      
      state_mut.log.range_min = i;

      // y here is only used for anchoring purposes
      // if the message happens to extend off-screen, we want to scroll to
      // accommodate it if/when the user selects it, so we'll assign it a
      // 'fake' value here
      anchors.insert(i, Anchor {
        // offset is the y offset of the first line of the message
        offset: max(0, end_y as isize - (y_pos as isize - lines.len() as isize + 1)) as u16,
        height: Some(lines.len() as u16)
      });

      for line in lines.iter().rev() {
        cursor.goto(0, y_pos as u16)?;
        terminal.clear(ClearType::CurrentLine)?;
        terminal.write(line)?;

        if y_pos == 0 {
          // we've reached the top
          break 'outer_up;
        } else {
          // still some room left
          y_pos -= 1;
        }
      }
    }

    // attempt to clear out any remaining empty space at the top (case #1)
    if y_pos > 0 {
      cursor.goto(0, y_pos)?;
      terminal.clear(ClearType::CurrentLine)?;
      terminal.clear(ClearType::FromCursorUp)?;
    }
  } else if start_y > 0 {
    // top clearing case #2
    cursor.goto(0, start_y - 1)?;
    terminal.clear(ClearType::CurrentLine)?;
    terminal.clear(ClearType::FromCursorUp)?;
  }

  Ok(())
}

pub fn render(
  mut state: RcState, terminal: &Terminal, cursor: &TerminalCursor
) -> Result<RcState, Box<Error>> {
  // ugly dancing around the borrow checker
  // the &mut needs to be dropped so we can return the new state
  {
    let state_mut = Rc::make_mut(&mut state);
    render_int(state_mut, terminal, cursor)?;
  }

  Ok(state)
}

pub mod actions {
  use super::*;

  /// Moves the current selection by some number of entries
  ///
  /// Positive amounts move the selection up, i.e. toward earlier messages,
  /// while negative amounts move the down, toward the latest message
  pub fn move_selection(state: RcState, amount: isize) -> RcState {
    if amount == 0 {
      return state;
    }

    let filtered_entries = &state.filtered_entries.borrow();

    // make a hard clone of state to play around in, otherwise we'll have to
    // jump through hoops to return later
    let mut state = (*state).clone();

    if filtered_entries.len() == 0 {
      // there's nothing to select
      return Rc::new(state);
    }

    let desired_index = if let Some(current) = &state.log.selection {
      let new = max(current.rel_index as isize - amount, 0) as usize;
      if new >= filtered_entries.len() {
        state.log.selection = None;

        return Rc::new(state);
      }

      new
    } else {
      // no selection means we're already anchored at the bottom, so moving
      // further down is a no-op
      if amount < 0 {
        state.log.selection = None;
        return Rc::new(state);
      }

      max(filtered_entries.len() as isize - amount, 0) as usize
    };

    if desired_index < state.log.range_min {
      // selected message is off-screen and early/above
      state.log.selection = Some(Selection {
        rel_index: desired_index,
        anchor: Anchor { offset: state.height - 1, height: None }
      });
    } else if desired_index > state.log.range_max {
      // selected message is off-screen and later/below
      // anchor to the height; this is definitely incorrect, but we don't want
      // to render the message here just to determine how many lines it spans
      // the renderer will adjust the selection if (when) it notices that it's
      // out of bounds
      state.log.selection = Some(Selection {
        rel_index: desired_index,
        anchor: Anchor { offset: 0, height: None }
      });
    } else {
      // selected message is already on-screen
      let anchor = state.log.anchors.borrow()[&desired_index];
      let anchor_height = anchor.height.unwrap_or(0);

      // if the anchor is partially off-screen (i.e. too high up), nudge in the
      // right direction
      let offset = if anchor.offset > state.height - 1 {
        // message extends upward
        state.height - 1
      } else if (anchor.offset as isize) - (anchor_height as isize) < 0 {
        // message extends downward
        anchor_height
      } else {
        anchor.offset
      };

      state.log.selection = Some(Selection {
        rel_index: desired_index,
        anchor: Anchor {
          offset,
          height: anchor.height
        }
      });
    }

    Rc::new(state)
  }

  /// Moves the selection to the given index, moving the viewport the minimum
  /// amount required to put it in view.
  ///
  /// Note that index is relative i.e. filtered entries (if any)
  pub fn move_selection_to_index(state: RcState, index: usize) -> RcState {
    let amount = if let Some(selection) = state.log.selection {
      selection.rel_index as isize - index as isize
    } else {
      (state.filtered_entries.borrow().len() - index) as isize
    };

    move_selection(state, amount)
  }

  pub fn move_selection_to_top(state: RcState) -> RcState {
    // note that the index given no selection deliberately +1 from the true last
    // index, as 1 selection up from empty will select the last message
    let index = state.log.selection
      .map_or(state.filtered_entries.borrow().len(), |s| s.rel_index);

    move_selection(state, index as isize)
  }

  pub fn move_selection_page_up(state: RcState) -> RcState {
    if let Some(selection) = state.log.selection {
      if selection.rel_index == 0 {
        // no-op
        return state;
      }

      if selection.rel_index == state.log.range_min {
        // move up a page, keeping at least one line of this old selection
        // visible to give the user some context

        // we can't move further than this
        let max_height = state.height as isize - 2;
        let mut running_height = 0;
        let mut running_count = 0;

        loop {
          let next_height = measure_entry(
            Rc::clone(&state), selection.rel_index - running_count - 1
          ) as isize;
          if running_height as isize + next_height > max_height {
            break;
          }

          running_height += next_height;
          running_count += 1;

          // avoid subtraction overflows...
          if running_count >= selection.rel_index {
            break;
          }
        }

        move_selection(Rc::clone(&state), running_count as isize)
      } else {
        // move to the top of the current page
        move_selection(
          Rc::clone(&state),
          selection.rel_index as isize - state.log.range_min as isize
        )
      }
    } else {
      // start the selection at the top of the current page
      move_selection(
        Rc::clone(&state),
        state.filtered_entries.borrow().len() as isize - state.log.range_min as isize
      )
    }
  }

  pub fn move_selection_page_down(state: RcState) -> RcState {
    if let Some(selection) = state.log.selection {
      let sel_index = selection.rel_index;
      let filtered_len = state.filtered_entries.borrow().len();

      if sel_index == filtered_len - 1 {
        // clear the selection
        return move_selection(state, -1);
      }

      if selection.rel_index == state.log.range_max {
        // move down a page, keeping at least one line of this old selection
        // visible to give the user some context

        // we can't move further than this
        let max_height = state.height as isize - 2;
        let mut running_height = 0;
        let mut running_count = 0;

        loop {
          let next_height = measure_entry(
            Rc::clone(&state), sel_index + running_count + 1
          ) as isize;
          if running_height as isize + next_height > max_height {
            break;
          }

          running_height += next_height;
          running_count += 1;

          if sel_index + running_count + 1 >= filtered_len {
            break;
          }
        }

        move_selection(Rc::clone(&state), -(running_count as isize))
      } else {
        // move to the bottom of the current page
        move_selection(
          Rc::clone(&state),
          sel_index as isize - state.log.range_max as isize
        )
      }
    } else {
      // no-op
      state
    }
  }

  pub fn clear_selection(mut state: RcState) -> RcState {
    Rc::make_mut(&mut state).log.selection = None;

    state
  }
}
