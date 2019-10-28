// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::marker::Sized;

use regex::Regex;
use simple_error::{SimpleError, SimpleResult};

use crate::parser::Message;

pub trait Filter {
  fn new(query: &str, inverted: bool) -> SimpleResult<Self> where Self: Sized;

  /// Determines if the filter is inverted
  fn inverted(&self) -> bool;

  /// Determines if the given message matches the filter without checking if the
  /// filter is inverted or not
  fn filter_pass(&self, message: &Message) -> bool;

  /// Determines if the given matches the filter, inverting the result if
  /// configured to do so.
  fn filter(&self, message: &Message) -> bool {
    let pass = self.filter_pass(message);

    if self.inverted() {
      !pass
    } else {
      pass
    }
  }
}

#[derive(Debug, Copy, Clone)]
pub enum FilterMode {
  Text,
  Regex
}

impl FilterMode {
  pub fn parse(self, filter: &str, inverted: bool) -> SimpleResult<Box<dyn Filter>> {
    Ok(match self {
      FilterMode::Text => Box::new(FullTextFilter::new(filter, inverted)?),
      FilterMode::Regex => Box::new(RegexFilter::new(filter, inverted)?)
    })
  }

  /// Given a FilterMode, return a different FilterMode (e.g. toggling between
  /// modes)
  pub fn next(self) -> FilterMode {
    // will probably need to be smarter if more modes are added
    match self {
      FilterMode::Text => FilterMode::Regex,
      FilterMode::Regex => FilterMode::Text
    }
  }

  pub fn name(self) -> &'static str {
    match self {
      FilterMode::Text => "text",
      FilterMode::Regex => "regex"
    }
  }
}

pub struct FullTextFilter {
  query: String,
  inverted: bool
}

impl Filter for FullTextFilter {
  fn new(query: &str, inverted: bool) -> SimpleResult<FullTextFilter> {
    Ok(FullTextFilter {
      query: query.to_lowercase(),
      inverted
    })
  }

  fn filter_pass(&self, message: &Message) -> bool {
    if message.kind.to_string().to_lowercase().contains(&self.query) {
      return true;
    }

    if let Some(level) = message.level {
      if level.to_string().to_lowercase().contains(&self.query) {
        return true;
      }
    }

    if let Some(text) = &message.text {
      if text.to_lowercase().contains(&self.query) {
        return true;
      }
    }

    for (k, v) in &message.metadata {
      if k.to_lowercase().contains(&self.query) {
        return true;
      }

      if v.to_string().to_lowercase().contains(&self.query) {
        return true;
      }
    }

    false
  }

  fn inverted(&self) -> bool {
    self.inverted
  }
}

pub struct RegexFilter {
  re: Regex,
  inverted: bool
}

impl Filter for RegexFilter {
  fn new(expr: &str, inverted: bool) -> SimpleResult<Self> {
    Regex::new(&expr)
      .map_err(SimpleError::from)
      .map(|re| RegexFilter { re, inverted })
  }

  fn filter_pass(&self, message: &Message) -> bool {
    if self.re.find(&message.kind.to_string().to_lowercase()).is_some() {
      return true;
    }

    if let Some(level) = message.level {
      if self.re.find(&level.to_string().to_lowercase()).is_some() {
        return true;
      }
    }

    if let Some(text) = &message.text {
      if self.re.find(&text).is_some() {
        return true;
      }
    }

    for (k, v) in &message.metadata {
      if self.re.find(k).is_some() {
        return true;
      }

      if self.re.find(&v.to_string()).is_some() {
        return true;
      }
    }

    false
  }

  fn inverted(&self) -> bool {
    self.inverted
  }
}
