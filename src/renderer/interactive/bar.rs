// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::error::Error;
use std::rc::Rc;

use crossterm::{Terminal, TerminalCursor, KeyEvent};

use super::{RcState, InputAction};
use super::log;
use super::status_bar;
use super::search_bar;
use super::filter_bar;

#[derive(Copy, Clone)]
pub enum BarType {
  Status,
  Filter,
  Search
}

#[derive(Clone)]
pub struct BarState {
  pub active: BarType,
}

impl BarState {
  pub fn new() -> Self {
    BarState {
      active: BarType::Status,
    }
  }
}

pub fn render(
  state: RcState, terminal: &Terminal, cursor: &TerminalCursor
) -> Result<RcState, Box<dyn Error>> {
  let renderer = match state.bar.active {
    BarType::Status => status_bar::render,
    BarType::Filter => filter_bar::render,
    BarType::Search => search_bar::render
  };

  renderer(state, terminal, cursor)
}

/// handles global input (e.g. ctrl-q, scrolling)
fn input_global(mut state: RcState, key: &KeyEvent) -> (RcState, InputAction) {
  state = match key {
    KeyEvent::Ctrl('q') => return (state, InputAction::Exit),
    KeyEvent::Up => log::actions::move_selection(state, 1),
    KeyEvent::Down => log::actions::move_selection(state, -1),
    KeyEvent::Home => log::actions::move_selection_to_top(state),
    KeyEvent::End => log::actions::clear_selection(state),
    KeyEvent::PageUp => log::actions::move_selection_page_up(state),
    KeyEvent::PageDown => log::actions::move_selection_page_down(state),
    _ => return (state, InputAction::Unhandled)
  };

  (state, InputAction::Rerender)
}

pub fn input(state: RcState, key: KeyEvent) -> (RcState, InputAction) {
  let (state, action) = input_global(state, &key);
  if action != InputAction::Unhandled {
    return (state, action);
  }

  let handler = match state.bar.active {
    BarType::Status => status_bar::input,
    BarType::Filter => filter_bar::input,
    BarType::Search => search_bar::input
  };

  handler(state, &key)
}

pub mod actions {
  use super::*;

  pub fn set_active(mut state: RcState, active: BarType) -> RcState {
    let state_mut = Rc::make_mut(&mut state);
    state_mut.bar.active = active;

    state
  }
}
