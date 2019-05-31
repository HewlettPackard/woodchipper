// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use regex::Regex;

pub fn clean_path(path: &str) -> String {
  lazy_static! {
    static ref RE: Regex = Regex::new(r"[/\\]").unwrap();
  }

  // this looks stupid but avoids excessive Vec allocations and saves ~3% total
  // runtime vs using iterators
  // this could be further reduced by splitting by only one path separator and
  // avoiding regexes, but supporting both formats seems worthwhile
  let mut last_a: Option<&str> = None;
  let mut last_b: Option<&str> = None;
  let mut last_c: Option<&str> = None;

  for part in RE.split(path) {
    if last_a.is_none() {
      last_a = Some(part);
    } else if last_b.is_none() {
      last_b = last_a;
      last_a = Some(part)
    } else {
      last_c = last_b;
      last_b = last_a;
      last_a = Some(part);
    }
  }

  let mut buf = String::new();
  if let Some(a) = last_a {
    buf.push_str(a);

    if let Some(b) = last_b {
      buf.insert(0, '/');
      buf.insert_str(0, b);

      if let Some(c) = last_c {
        buf.insert(0, '/');
        buf.insert_str(0, c);
      }
    }
  }

  buf
}
