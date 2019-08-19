// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::error::Error;

use crossterm::{Terminal, TerminalCursor, KeyEvent, ClearType};

use crate::clip::{clip, clipboard_enabled};
use crate::renderer::interactive::InputAction;
use crate::renderer::interactive::bar::{self, BarType};
use crate::renderer::interactive::log;
use crate::renderer::interactive::state::RcState;
use crate::renderer::interactive::state::actions as state_actions;
use crate::renderer::plain::plain_render;

fn format_left(state: &RcState) -> (usize, String) {
  let mut buf = String::new();
  buf.push_str("q: quit | f: filter | /: find");

  if clipboard_enabled() {
    if state.log.selection.is_some() {
      buf.push_str(" | c: copy msg");
    }

    buf.push_str(" | S-c: copy screen");
  }

  if !state.filters.borrow().is_empty() {
    buf.push_str(" | p: pop filter");
  }

  (buf.len(), buf)
}

pub fn format_right(state: &RcState) -> (usize, String) {
  let len_filters = state.filters.borrow().len();
  let len_entries = state.entries.borrow().len();
  let len_filtered_entries = state.filtered_entries.borrow().len();

  let eof = if state.eof { " (eof)" } else { "" };
  let filters = if len_filters == 0 {
    "".to_string()
  } else {
    format!(
      " ({} filter{}, {} total)",
      len_filters,
      if len_filters  == 1 { "" } else { "s" },
      len_entries
    )
  };

  let count = if let Some(selection) = state.log.selection {
    format!("{} / {}", selection.rel_index + 1, len_filtered_entries)
  } else {
    format!("{}", len_filtered_entries)
  };

  // this will need to change if any parts are styled in the future
  let right = format!("{}{}{}", count, filters, eof);
  (right.len(), right)
}

pub fn render(
  state: RcState, terminal: &Terminal, cursor: &TerminalCursor
) -> Result<RcState, Box<dyn Error>> {
  let (left_len, left) = format_left(&state);
  let (right_len, right) = format_right(&state);

  let profile = &state.config.style.selected;
  let style = profile.get_base();

  // prioritize showing the right-side content as the left is just help text
  let mut buf = String::new();

  let combined_len = left_len + right_len;
  if let Some(spacer) = state.width.checked_sub(combined_len as u16) {
    // room for both sides
    buf.push_str(&left);
    for _ in 0..spacer {
      buf.push(' ');
    }

    buf.push_str(&right);
  } else if let Some(spacer) = state.width.checked_sub(right_len as u16) {
    // only room for right
    for _ in 0..spacer {
      buf.push(' ');
    }

    buf.push_str(&right);
  } else {
    // just fill
    for _ in 0..state.width {
      buf.push(' ');
    }
  }

  cursor.hide()?;
  cursor.goto(0, state.height - 1)?;
  terminal.clear(ClearType::CurrentLine)?;
  terminal.write(style.paint(&buf))?;

  Ok(state)
}

pub fn input(mut state: RcState, key: &KeyEvent) -> (RcState, InputAction) {
  state = match key {
    KeyEvent::Esc => {
      if state.log.selection.is_some() {
        log::actions::clear_selection(state)
      } else {
        return (state, InputAction::Exit)
      }
    }
    KeyEvent::Char(c) => match c {
      'q' => return (state, InputAction::Exit),
      '|' | 'f' => bar::actions::set_active(state, BarType::Filter),
      '/' => bar::actions::set_active(state, BarType::Search),
      'p' => {
        if state.filters.borrow().is_empty() {
          state_actions::internal(state, "no filters to remove")
        } else {
          state_actions::pop_filter(state)
        }
      },
      'c' => actions::copy_selection(state),
      'C' => actions::copy_view(state),
      _ => return (state, InputAction::Unhandled)
    },
    KeyEvent::Ctrl(c) => match c {
      'c' => {
        if state.log.selection.is_some() {
          actions::copy_selection(state)
        } else {
          return (state, InputAction::Exit)
        }
      },
      'f' => bar::actions::set_active(state, BarType::Search),
      _ => return (state, InputAction::Unhandled)
    }
    _ => return (state, InputAction::Unhandled)
  };

  (state, InputAction::Rerender)
}

pub mod actions {
  use super::*;

  pub fn copy_selection(state: RcState) -> RcState {
    if !clipboard_enabled() {
      return state;
    }

    if let Some(selection) = state.log.selection {
      let plain = {
        let entry = &state.filtered_entries.borrow()[selection.rel_index];
        plain_render(&entry.entry.upgrade().unwrap())
      }.join("\n");

      // TODO: handle unset weak ref
      match clip(plain) {
        Ok(()) => state_actions::internal(state, "copied message to clipboard"),
        Err(e) => state_actions::internal(
          state, &format!("error writing to clipboard: {:?}", e)
        )
      }
    } else {
      state_actions::internal(state, "no message is selected")
    }
  }

  pub fn copy_view(state: RcState) -> RcState {
    if !clipboard_enabled() {
      return state;
    }

    let mut lines = 0;
    let mut buf = String::new();
    for i in state.log.range_min..=state.log.range_max {
      let entry = &state.filtered_entries.borrow()[i];

      // TODO: handle unset weak ref
      for line in plain_render(&entry.entry.upgrade().unwrap()) {
        buf.push_str(&line);
        buf.push('\n');
        lines += 1;
      }
    }

    match clip(buf) {
      Ok(()) => state_actions::internal(
        state, &format!("copied {} lines to clipboard", lines)
      ),
      Err(e) => state_actions::internal(
        state, &format!("error writing to clipboard: {:?}", e)
      )
    }

  }
}