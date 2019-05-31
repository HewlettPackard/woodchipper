// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use simple_error::{SimpleError, SimpleResult};

#[cfg(target_env = "musl")]
use subprocess::{Exec, Redirection};

#[cfg(not(target_env = "musl"))]
use clipboard::{ClipboardContext, ClipboardProvider};

#[cfg(not(target_env = "musl"))]
fn clip_all(text: String) -> SimpleResult<()> {
  let mut ctx: ClipboardContext = match ClipboardProvider::new() {
    Ok(ctx) => ctx,
    Err(e) => return Err(SimpleError::new(e.to_string()))
  };

  match ctx.set_contents(text) {
    Ok(()) => Ok(()),
    Err(e) => Err(SimpleError::new(e.to_string()))
  }
}

#[cfg(target_env = "musl")]
fn clip_xclip(text: String) -> SimpleResult<()> {
  let result = Exec::cmd("xclip")
    .args(&["-sel", "clip"])
    .stdin(text.as_str())
    .stdout(Redirection::Merge)
    .capture()
    .map_err(SimpleError::from)?;

  if !result.success() {
    Err(SimpleError::new("xclip returned an error"))
  } else {
    Ok(())
  }
}

pub fn clip(text: String) -> SimpleResult<()> {
  if !clipboard_enabled() {
    return Ok(());
  }

  #[cfg(target_env = "musl")]
  let clip_fn = clip_xclip;
 
  #[cfg(not(target_env = "musl"))]
  let clip_fn = clip_all;

  clip_fn(text)
}

#[inline]
pub fn clipboard_enabled() -> bool {
  cfg!(feature = "wd-clipboard")
}
