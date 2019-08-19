// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::error::Error;

use crossterm::{Terminal, TerminalCursor, KeyEvent};

use crate::style::Styler;
use super::InputAction;
use super::state::RcState;

#[derive(Clone)]
pub struct TextBuffer {
  pub input: String,
  pub position: usize,
  pub styler: Option<Styler>
}

impl TextBuffer {
  pub fn new() -> Self {
    TextBuffer {
      input: String::new(),
      position: 1,
      styler: None
    }
  }

  pub fn with_styler(mut self, styler: Option<Styler>) -> Self {
    self.styler = styler;

    self
  }
}

pub enum TextInputAction {
  /// user has attempted to submit the text input
  /// note that the parent component is responsible for validating and clearing
  /// the input
  Submit(InputAction, String),

  /// text content was updated
  Update(InputAction),

  /// user has ended input with no content
  Exit(InputAction),

  /// some pass-through action
  Action(InputAction)
}

pub fn input(
  mut state: TextBuffer, key: &KeyEvent
) -> (TextBuffer, TextInputAction) {
  let action = match key {
    KeyEvent::Esc => {
      state = actions::clear_input(state);
      TextInputAction::Exit(InputAction::Rerender)
    },
    KeyEvent::Backspace => {
      if state.input.chars().next().is_some() {
        state = actions::pop_input_back(state);
        TextInputAction::Update(InputAction::Rerender)
      } else {
        TextInputAction::Exit(InputAction::Rerender)
      }
    },
    KeyEvent::Delete => {
      state = actions::pop_input_forward(state);
      TextInputAction::Update(InputAction::Rerender)
    },
    KeyEvent::Char('\n') => {
      let input = state.input.to_string();
      TextInputAction::Submit(InputAction::Rerender, input)
    },
    KeyEvent::Left => {
      state = actions::cursor_left(state);
      TextInputAction::Action(InputAction::Rerender)
    },
    KeyEvent::Right => {
      state = actions::cursor_right(state);
      TextInputAction::Action(InputAction::Rerender)
    },
    KeyEvent::Char(c) => {
      state = actions::push_input(state, *c);
      TextInputAction::Update(InputAction::Rerender)
    },
    _ => TextInputAction::Action(InputAction::Unhandled)
  };

  (state, action)
}

pub fn render(
  state: RcState, text: &TextBuffer,
  terminal: &Terminal, cursor: &TerminalCursor,
  x: u16, y: u16
) -> Result<(), Box<dyn Error>> {
  // TODO: need x, y as crossterm's cursor.pos() is currently broken:
  // https://github.com/TimonPost/crossterm/issues/122
  // we can use pos and goto once fixed to let the caller position the cursor
  // before calling this render()

  let out_text = if let Some(styler) = &text.styler {
    let style = styler(&state.config.style);
    style.paint(&text.input).to_string()
  } else {
    text.input.clone()
  };

  cursor.show()?;
  cursor.goto(x, y)?;

  terminal.write(&out_text)?;

  cursor.goto(x + text.position as u16 - 1, y)?;

  Ok(())
}

pub mod actions {
  use super::*;

  pub fn clear_input(mut state: TextBuffer) -> TextBuffer {
    state.input.clear();
    state.position = 1;

    state
  }

  pub fn pop_input_back(mut state: TextBuffer) -> TextBuffer {
    let pos = state.position;
    let len = state.input.chars().count();

    if pos > len {
      // cursor is at the end of input (or somehow beyond it)
      if state.input.pop().is_some() {
        state.position = len;
      } else {
        state.position = 1;
      }
    } else if pos > 1 {
      // cursor is somewhere in the middle
      state.input.remove(pos - 2);
      state.position -= 1;
    } else {
      // ignore if cursor is at the start *and* there's text in the buffer
    }

    state
  }

  pub fn pop_input_forward(mut state: TextBuffer) -> TextBuffer {
    let pos = state.position;
    let len = state.input.chars().count();

    if pos > len {
      // cursor is at the end of input (or somehow beyond it), ignore
      state
    } else if len > 0 {
      state.input.remove(pos - 1);

      state
    } else {
      // ignore if cursor is at the start *and* there's text in the buffer
      state
    }
  }

  pub fn push_input(mut state: TextBuffer, c: char) -> TextBuffer {
    let pos = state.position;
    let len = state.input.chars().count();

    if pos > len {
      state.input.push(c);
    } else {
      state.input.insert(pos - 1, c);
    }

    state.position += 1;

    state
  }

  pub fn cursor_left(mut state: TextBuffer) -> TextBuffer {
    let pos = state.position;
    if pos > 1 {
      state.position -= 1;
    }

    state
  }

  pub fn cursor_right(mut state: TextBuffer) -> TextBuffer {
    let pos = state.position;
    if pos <= state.input.chars().count() {
      state.position += 1;
    }

    state
  }
}
