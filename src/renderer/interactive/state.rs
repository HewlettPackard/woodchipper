// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use crate::config::Config;
use crate::filter::Filter;
use crate::renderer::types::*;

use super::log::LogState;
use super::bar::BarState;
use super::filter_bar::FilterBarState;
use super::search_bar::SearchBarState;

pub struct FilteredEntry {
  pub index: usize,
  pub entry: Weak<MessageEntry>,
}

/// shared state between all components
/// this struct is semi-immutable: each action should return a new clone, but
/// certain fields (entries and filtered_entries) are shared as cloning would
/// be very expensive
/// for perf reasons, this should generally be passed around inside a Cow
/// (see CowState)
#[derive(Clone)]
pub struct RenderState {
  pub config: Arc<Config>,

  pub width: u16,
  pub height: u16,

  /// A list of all parsed entries read from the input.
  ///
  /// This list may be quite large and is shared between otherwise immutable
  /// RenderState instances for performance.
  pub entries: Rc<RefCell<Vec<Rc<MessageEntry>>>>,

  /// A list of filters used to generated `filtered_entries` from `entries`
  pub filters: Rc<RefCell<Vec<Box<dyn Filter>>>>,

  /// A Vec of entries filtered from the main list..
  ///
  /// This list contains the subset of entries requested by the user
  pub filtered_entries: Rc<RefCell<Vec<FilteredEntry>>>,

  /// A cached temporary filter representing the user's current filter input,
  /// if it exists and is valid.
  ///
  /// in a refcell because we can't clone all filter types :/
  pub highlight_filter: Option<Rc<Box<dyn Filter>>>,

  /// If true, input EoF has been reached
  pub eof: bool,

  pub log: LogState,
  pub bar: BarState,
  pub filter: FilterBarState,
  pub search: SearchBarState
}

/// A RenderState wrapped in a Cow for perf reasons
pub type RcState = Rc<RenderState>;

// TODO should RenderState be passed to actions in a Cow?
// then we can skip cloning when nothing is changed, or if we only write to
// one of the refcell fields
impl RenderState {
  pub fn new(config: Arc<Config>) -> Self {
    RenderState {
      config,

      width: 0,
      height: 0,

      entries: Rc::new(RefCell::new(Vec::new())),
      filters: Rc::new(RefCell::new(Vec::new())),
      filtered_entries: Rc::new(RefCell::new(Vec::new())),

      highlight_filter: None,

      eof: false,

      log: LogState::new(),
      bar: BarState::new(),
      filter: FilterBarState::new(),
      search: SearchBarState::new()
    }
  }
}

pub fn filter_pass(state: RcState, entry: &MessageEntry) -> bool {
  let filters = state.filters.borrow();
  if filters.is_empty() {
    return true;
  }

  for filter in filters.iter() {
    if !filter.filter(&entry.message) {
      return false;
    }
  }

  true
}

pub mod actions {
  use super::*;

  pub fn add_filter(mut state: RcState, filter: Box<dyn Filter>) -> RcState {
    let state_mut = Rc::make_mut(&mut state);
    state_mut.filters.borrow_mut().push(filter);

    // TODO: figure out how to keep the selection while adjusting filters
    state_mut.log.selection = None;

    *state.filtered_entries.borrow_mut() = state.entries.borrow().iter()
      .enumerate()
      .filter(|(_, e)| filter_pass(Rc::clone(&state), e))
      .map(|(i, e)| FilteredEntry {
        index: i,
        entry: Rc::downgrade(e)
      })
      .collect();

    state
  }

  pub fn pop_filter(mut state: RcState) -> RcState {
    let state_mut = Rc::make_mut(&mut state);
    state_mut.log.selection = None;

    state.filters.borrow_mut().pop();

    let new_filtered = if state.filters.borrow().is_empty() {
      state.entries.borrow().iter()
        .enumerate()
        .filter(|(_, e)| filter_pass(Rc::clone(&state), e))
        .map(|(i, e)| FilteredEntry {
          index: i,
          entry: Rc::downgrade(e)
        })
        .collect()

    } else {
      state.entries.borrow().iter()
        .enumerate()
        .filter(|(_, e)| filter_pass(Rc::clone(&state), e))
        .map(|(i, e)| FilteredEntry {
          index: i,
          entry: Rc::downgrade(e)
        })
        .collect()
    };

    *state.filtered_entries.borrow_mut() = new_filtered;

    state
  }

  /// updates the temp filter based on user input
  pub fn set_highlight_filter(
    mut state: RcState, filter: Option<Rc<Box<dyn Filter>>>
  ) -> RcState {
    let state_mut = Rc::make_mut(&mut state);

    if let Some(filter) = filter {
      state_mut.highlight_filter = Some(filter);
    } else {
      state_mut.highlight_filter = None;
    }

    state
  }

  pub fn add_entry(state: RcState, entry: MessageEntry) -> RcState {
    {
      // this mut borrow needs to be dropped so we can return state
      let mut entries = state.entries.borrow_mut();

      if filter_pass(Rc::clone(&state), &entry) {
        entries.push(Rc::new(entry));
        state.filtered_entries.borrow_mut().push(FilteredEntry {
          index: entries.len() - 1,
          entry: Rc::downgrade(&entries[entries.len() - 1]),
        });
      } else {
        entries.push(Rc::new(entry));
      }
    }

    state
  }

  pub fn internal(state: RcState, text: &str) -> RcState {
    add_entry(state, MessageEntry::internal(text))
  }

  pub fn set_eof(mut state: RcState, eof: bool) -> RcState {
    let state_mut = Rc::make_mut(&mut state);
    state_mut.eof = eof;

    state
  }
}
