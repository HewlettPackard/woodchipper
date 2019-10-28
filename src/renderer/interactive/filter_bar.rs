// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::error::Error;
use std::rc::Rc;

use crossterm::{Terminal, TerminalCursor, KeyEvent, ClearType};

use crate::filter::FilterMode;
use crate::style::{StyleProfileKind, styler_base, styler_error};

use super::state::{self, RcState};
use super::state::actions as state_actions;
use super::bar::{self, BarType};
use super::text::{self, TextBuffer, TextInputAction};
use super::InputAction;

#[derive(Clone)]
pub struct FilterBarState {
  mode: FilterMode,
  text: TextBuffer,
  inverted: bool
}

impl FilterBarState {
  pub fn new() -> Self {
    let styler = styler_base(StyleProfileKind::Selected);

    FilterBarState {
      mode: FilterMode::Regex,
      text: TextBuffer::new().with_styler(Some(styler)),
      inverted: false
    }
  }
}

fn format_right(state: &RcState) -> String {
  if state.width < 80 {
    let inv = if state.filter.inverted { "y" } else { "n" };

    format!(
      "| m: {} (C-r), i: {} (C-e)",
      state.filter.mode.name(),
      inv
    )
  } else {
    let inv = if state.filter.inverted { "yes" } else { "no" };

    format!(
      "| mode: {} (C-r), invert: {} (C-e)",
      state.filter.mode.name(),
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
  cursor.goto(0, state.height - 1)?;

  terminal.write(&style.paint("filter > ").to_string())?;
  text::render(
    Rc::clone(&state), &state.filter.text,
    terminal, cursor,
    9, state.height - 1
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

  let text_state = state_mut.filter.text.clone();
  let (text_state, action) = text::input(text_state, key);
  state_mut.filter.text = text_state;

  (state, action)
}

pub fn input(mut state: RcState, key: &KeyEvent) -> (RcState, InputAction) {
  let (new_state, action) = handle_text_input(state, key);
  state = new_state;

  let input_action = match action {
    TextInputAction::Action(a) => a,
    TextInputAction::Exit(a) => {
      state = actions::update_highlight(state);
      state = actions::update_style(state);
      state = bar::actions::set_active(state, BarType::Status);
      a
    },
    TextInputAction::Submit(a, input) => {
      match state.filter.mode.parse(&input, state.filter.inverted) {
        Ok(filter) => {
          state = actions::clear_input(state);
          state = bar::actions::set_active(state, BarType::Status);
          state = actions::update_highlight(state);
          state = actions::update_style(state);
          state = state::actions::add_filter(state, filter);
        },
        Err(e) => state = state::actions::internal(
          state, &format!("invalid filter: {:?}", e)
        )
      }

      a
    },
    TextInputAction::Update(a) => {
      state = actions::update_highlight(state);
      state = actions::update_style(state);
      a
    }
  };

  let final_action = match input_action {
    InputAction::Unhandled => match key {
      KeyEvent::Ctrl('r') => {
        state = actions::next_mode(state);
        state = actions::update_highlight(state);
        state = actions::update_style(state);

        InputAction::Rerender
      },
      KeyEvent::Ctrl('e') => {
        state = actions::toggle_inverted(state);
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

  pub fn update_highlight(state: RcState) -> RcState {
    let input = &state.filter.text.input;

    let new_filter = if input.is_empty() {
      None
    } else if let Ok(parsed) = state.filter.mode.parse(&input, state.filter.inverted) {
      Some(Rc::new(parsed))
    } else {
      None
    };

    state_actions::set_highlight_filter(state, new_filter)
  }

  pub fn update_style(mut state: RcState) -> RcState {
    let state_mut = Rc::make_mut(&mut state);

    let input = &state_mut.filter.text.input;
    let mode = &state_mut.filter.mode;

    let styler = if input.is_empty() || mode.parse(input, state_mut.filter.inverted).is_ok() {
      styler_base(StyleProfileKind::Selected)
    } else {
      styler_error(StyleProfileKind::Selected)
    };

    state_mut.filter.text.styler = Some(styler);

    state
  }

  /// wrapper for text::actions::clear_input to expose it as a standard action
  pub fn clear_input(mut state: RcState) -> RcState {
    let state_mut = Rc::make_mut(&mut state);

    state_mut.filter.text = text::actions::clear_input(
      state_mut.filter.text.clone()
    );

    state
  }

  pub fn next_mode(mut state: RcState) -> RcState {
    let state_mut = Rc::make_mut(&mut state);
    state_mut.filter.mode = state_mut.filter.mode.next();

    state
  }

  pub fn toggle_inverted(mut state: RcState) -> RcState {
    let state_mut = Rc::make_mut(&mut state);
    state_mut.filter.inverted = !state_mut.filter.inverted;

    state
  }
}
