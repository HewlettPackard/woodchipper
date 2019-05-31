// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::marker::Sized;

use jsonpath::Selector;
use regex::Regex;
use serde_json;
use simple_error::{SimpleError, SimpleResult};

use crate::parser::Message;

pub trait Filter {
  fn new(query: &str) -> SimpleResult<Self> where Self: Sized;
  fn filter(&self, message: &Message) -> bool;
}

#[derive(Debug, Copy, Clone)]
pub enum FilterMode {
  Text,
  Regex,
  Json
}

impl FilterMode {
  pub fn parse(self, filter: &str) -> SimpleResult<Box<Filter>> {
    Ok(match self {
      FilterMode::Text => Box::new(FullTextFilter::new(filter)?),
      FilterMode::Regex => Box::new(RegexFilter::new(filter)?),
      FilterMode::Json => Box::new(JsonPathFilter::new(filter)?)
    })
  }
}

pub struct FullTextFilter {
  query: String
}

impl Filter for FullTextFilter {
  fn new(query: &str) -> SimpleResult<FullTextFilter> {
    Ok(FullTextFilter {
      query: query.to_lowercase()
    })
  }

  fn filter(&self, message: &Message) -> bool {
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
}

pub struct RegexFilter {
  re: Regex
}

impl Filter for RegexFilter {
  fn new(expr: &str) -> SimpleResult<Self> {
    Regex::new(&expr)
      .map_err(SimpleError::from)
      .map(|re| RegexFilter { re })
  }

  fn filter(&self, message: &Message) -> bool {
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
}

pub struct JsonPathFilter {
  selector: Selector
}

impl Filter for JsonPathFilter {
  fn new(expression: &str) -> SimpleResult<Self> {
    Selector::new(expression)
      .map_err(SimpleError::from)
      .map(|selector| JsonPathFilter { selector })
  }

  fn filter(&self, message: &Message) -> bool {
    // partially re-serialize message to filter it...
    // if this fails (it shouldn't), ignore and move on
    let val = match serde_json::value::to_value(message) {
      Ok(val) => val,
      Err(_) => return false
    };

    // we don't actually care about the match value here, and just want to know
    // if the selector at least matches something
    self.selector.find(&val).next().is_some()
  }
}

