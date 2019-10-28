// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::error::Error;
use std::rc::Rc;

use crossterm::{Terminal, TerminalCursor, KeyEvent, ClearType};

use crate::filter::{Filter, FilterMode};
use crate::style::{StyleProfileKind, styler_base, styler_error};

use super::state::RcState;
use super::state::actions as state_actions;
use super::bar::{self, BarType};
use super::status_bar;
use super::log;
use super::text::{self, TextBuffer, TextInputAction};
use super::InputAction;

#[derive(Clone)]
pub struct SearchBarState {
  mode: FilterMode,
  text: TextBuffer,
  inverted: bool,
  filter: Option<Rc<Box<dyn Filter>>>
}

impl SearchBarState {
  pub fn new() -> Self {
    let styler = styler_base(StyleProfileKind::Selected);

    SearchBarState {
      mode: FilterMode::Regex,
      text: TextBuffer::new().with_styler(Some(styler)),
      inverted: false,
      filter: None
    }
  }
}

fn format_right(state: &RcState) -> String {
  if state.width < 80 {
    let inv = if state.search.inverted { "y" } else { "n" };

    format!(
      "| m: {} (C-r), i: {} (C-e)",
      state.search.mode.name(),
      inv
    )
  } else {
    let inv = if state.search.inverted { "yes" } else { "no" };

    format!(
      "| mode: {} (C-r), invert: {} (C-e)",
      state.search.mode.name(),
      inv
    )
  }
}

pub fn render(
  state: RcState, terminal: &Terminal, cursor: &TerminalCursor
) -> Result<RcState, Box<dyn Error>> {
  cursor.goto(0, state.height - 1)?;
  terminal.clear(ClearType::CurrentLine)?;

  let style = &state.config.style.selected.get_base();
  terminal.write(style.paint(" ".repeat(state.width as usize)))?;

  let (right_len, right) = status_bar::format_right(&state);
  if let Some(x) = state.width.checked_sub(right_len as u16) {
    cursor.goto(x, state.height - 1)?;
    terminal.write(style.paint(&right))?;
  }

  cursor.goto(0, state.height - 1)?;
  terminal.write(&style.paint("find > ").to_string())?;
  text::render(
    Rc::clone(&state), &state.search.text,
    terminal, cursor,
    7, state.height - 1
  )?;

  // note: this will cover up excessively long user input (text module should
  // support some form of horizontal scrolling?)
  let right = format_right(&state);
  let right_len = right.len();
  if let Some(col) = state.width.checked_sub(right_len as u16) {
    cursor.goto(col, state.height - 1)?;
    terminal.write(&style.paint(right))?;
  }

  Ok(state)
}

/// handles text component input in a pseudo-action
///
/// it doesn't /quite/ conform to the 'RcState in, RcState out' pattern so it
/// isn't explicitly an action
fn handle_text_input(
  mut state: RcState, key: &KeyEvent
) -> (RcState, TextInputAction) {
  let state_mut = Rc::make_mut(&mut state);

  let text_state = state_mut.search.text.clone();
  let (text_state, action) = text::input(text_state, key);
  state_mut.search.text = text_state;

  (state, action)
}

pub fn input(mut state: RcState, key: &KeyEvent) -> (RcState, InputAction) {
  let (new_state, action) = handle_text_input(state, key);
  state = new_state;

  let input_action = match action {
    TextInputAction::Action(a) => a,
    TextInputAction::Exit(a) => {
      state = actions::update_filter(state);
      state = actions::update_highlight(state);
      state = actions::update_style(state);
      state = bar::actions::set_active(state, BarType::Status);

      a
    },
    TextInputAction::Submit(a, _) => {
      state = actions::next_match(state, false);

      a
    },
    TextInputAction::Update(a) => {
      state = actions::update_filter(state);
      state = actions::next_match(state, true);

      state = actions::update_highlight(state);
      state = actions::update_style(state);

      a
    }
  };

  let final_action = match input_action {
    InputAction::Unhandled => match key {
      KeyEvent::Ctrl('p') => {
        state = actions::prev_match(state);

        InputAction::Rerender
      },
      KeyEvent::Ctrl('n') => {
        state = actions::next_match(state, false);

        InputAction::Rerender
      },
      KeyEvent::Ctrl('r') => {
        state = actions::next_mode(state);
        state = actions::update_filter(state);
        state = actions::next_match(state, true);
        state = actions::update_highlight(state);
        state = actions::update_style(state);

        InputAction::Rerender
      },
      KeyEvent::Ctrl('e') => {
        state = actions::toggle_inverted(state);
        state = actions::update_filter(state);
        state = actions::next_match(state, true);
        state = actions::update_highlight(state);
        state = actions::update_style(state);

        InputAction::Rerender
      },
      _ => InputAction::Unhandled
    },
    _ => input_action
  };

  (state, final_action)
}

pub mod actions {
  use super::*;

  /// Updates the search filter given current user input
  pub fn update_filter(mut state: RcState) -> RcState {
    let input = &state.search.text.input;

    let new_filter = if input.is_empty() {
      None
    } else if let Ok(parsed) = state.search.mode.parse(&input, state.search.inverted) {
      Some(Rc::new(parsed))
    } else {
      None
    };

    let state_mut = Rc::make_mut(&mut state);
    state_mut.search.filter = new_filter;

    state
  }

  /// Updates the log's highlight filter based on the current search filter,
  /// if any.
  pub fn update_highlight(state: RcState) -> RcState {
    let filter_clone = if let Some(filter) = &state.search.filter {
      Some(Rc::clone(&filter))
    } else {
      None
    };

    state_actions::set_highlight_filter(state, filter_clone)
  }

  /// Updates the text field's rendering styler based on the validation result
  /// of the filter.
  ///
  /// Invalid filters will be highlighted in red.
  pub fn update_style(mut state: RcState) -> RcState {
    let state_mut = Rc::make_mut(&mut state);

    let input = &state_mut.search.text.input;
    let mode = &state_mut.search.mode;

    let styler = if input.is_empty() || mode.parse(input, state_mut.search.inverted).is_ok() {
      styler_base(StyleProfileKind::Selected)
    } else {
      styler_error(StyleProfileKind::Selected)
    };

    state_mut.search.text.styler = Some(styler);

    state
  }

  /// Moves to the next (i.e. forward in time) matching entry.
  ///
  /// If `soft`, don't move if the currently-selected entry already matches,
  /// otherwise always moves to the next matching entry
  pub fn next_match(mut state: RcState, soft: bool) -> RcState {
    let filter = if let Some(filter) = &state.search.filter {
      filter
    } else {
      return state;
    };

    let max = state.filtered_entries.borrow().len();
    let min = if let Some(selection) = state.log.selection {
      // start from the selection, if any (but don't exceed the list)
      let offset = if !soft && selection.rel_index + 1 < max {
        // if not `soft`, start the search at the next entry
        1
      } else {
        // ... otherwise (if soft), start the search from the current entry
        0
      };

      selection.rel_index + offset
    } else {
      // otherwise, start from the beginning
      0
    };

    let mut index = None;

    {
      let filtered_entries = state.filtered_entries.borrow();

      // iter methods aren't quite sufficient here
      #[allow(clippy::needless_range_loop)]
      for i in min..max {
        if let Some(entry) = &filtered_entries[i].entry.upgrade() {
          if filter.filter(&entry.message) {
            index = Some(i);
            break;
          }
        }
      }
    }

    if let Some(index) = index {
      log::actions::move_selection_to_index(state, index)
    } else {
      // no match was found
      state = state_actions::internal(
        state,
        "reached end of log with no match"
      );

      log::actions::clear_selection(state)
    }
  }

  /// Moves to the previous (i.e. backward in time) matching entry.
  pub fn prev_match(mut state: RcState) -> RcState {
    let filter = if let Some(filter) = &state.search.filter {
      filter
    } else {
      return state;
    };

    let min = 0;
    let max = if let Some(selection) = state.log.selection {
      selection.rel_index
    } else {
      state.filtered_entries.borrow().len()
    };

    let mut index = None;
    {
      let filtered_entries = state.filtered_entries.borrow();
      for i in (min..max).rev() {
        if let Some(entry) = &filtered_entries[i].entry.upgrade() {
          if filter.filter(&entry.message) {
            index = Some(i);
            break;
          }
        }
      }
    }

    if let Some(index) = index {
      log::actions::move_selection_to_index(state, index)
    } else {
      // no match was found
      state = state_actions::internal(
        state,
        "reached beginning of log with no match"
      );

      log::actions::clear_selection(state)
    }
  }

  pub fn next_mode(mut state: RcState) -> RcState {
    let state_mut = Rc::make_mut(&mut state);
    state_mut.search.mode = state_mut.search.mode.next();

    state
  }

  pub fn toggle_inverted(mut state: RcState) -> RcState {
    let state_mut = Rc::make_mut(&mut state);
    state_mut.search.inverted = !state_mut.search.inverted;

    state
  }
}
