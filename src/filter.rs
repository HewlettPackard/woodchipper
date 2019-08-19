// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::marker::Sized;

use regex::Regex;
use simple_error::{SimpleError, SimpleResult};

use crate::parser::Message;

pub trait Filter {
  fn new(query: &str) -> SimpleResult<Self> where Self: Sized;
  fn filter(&self, message: &Message) -> bool;
}

#[derive(Debug, Copy, Clone)]
pub enum FilterMode {
  #[allow(dead_code)] Text,
  Regex
}

impl FilterMode {
  pub fn parse(self, filter: &str) -> SimpleResult<Box<dyn Filter>> {
    Ok(match self {
      FilterMode::Text => Box::new(FullTextFilter::new(filter)?),
      FilterMode::Regex => Box::new(RegexFilter::new(filter)?)
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
