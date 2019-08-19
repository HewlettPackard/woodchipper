// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::BTreeMap;
use std::error::Error;
use std::io::BufReader;
use std::fmt;
use std::fs::File;
use std::rc::Rc;
use std::str::FromStr;

use ansi_term::{Style, Color};
use regex::Regex;
use serde::Deserialize;
use serde::de::{self, Visitor, Unexpected, Deserializer};
use shellexpand;

use crate::classifier::ChunkKind;
use crate::parser::LogLevel;

struct ColorFromStr;

impl<'de> Visitor<'de> for ColorFromStr {
  type Value = Color;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("a string containing a hexidecimal RGB color")
  }

  fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    let color_part = s.trim_start_matches('#');
    if color_part.len() != 6 {
      return Err(de::Error::invalid_length(6, &self));
    }

    let r = match u8::from_str_radix(&color_part[0..2], 16) {
      Ok(r) => r,
      Err(_) => return Err(de::Error::invalid_value(Unexpected::Str(s), &self))
    };

    let g = match u8::from_str_radix(&color_part[2..4], 16) {
      Ok(g) => g,
      Err(_) => return Err(de::Error::invalid_value(Unexpected::Str(s), &self))
    };

    let b = match u8::from_str_radix(&color_part[4..6], 16) {
      Ok(b) => b,
      Err(_) => return Err(de::Error::invalid_value(Unexpected::Str(s), &self))
    };

    Ok(Color::RGB(r, g, b))
  }
}

fn de_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
  D: Deserializer<'de>
{
  deserializer.deserialize_str(ColorFromStr)
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct Base16 {
  #[serde(deserialize_with = "de_color")] base00: Color,
  #[serde(deserialize_with = "de_color")] base01: Color,
  #[serde(deserialize_with = "de_color")] base02: Color,
  #[serde(deserialize_with = "de_color")] base03: Color,
  #[serde(deserialize_with = "de_color")] base04: Color,
  #[serde(deserialize_with = "de_color")] base05: Color,
  #[serde(deserialize_with = "de_color")] base06: Color,
  #[serde(deserialize_with = "de_color")] base07: Color,
  #[serde(deserialize_with = "de_color")] base08: Color,
  #[serde(deserialize_with = "de_color")] base09: Color,
  #[serde(deserialize_with = "de_color")] base0A: Color,
  #[serde(deserialize_with = "de_color")] base0B: Color,
  #[serde(deserialize_with = "de_color")] base0C: Color,
  #[serde(deserialize_with = "de_color")] base0D: Color,
  #[serde(deserialize_with = "de_color")] base0E: Color,
  #[serde(deserialize_with = "de_color")] base0F: Color,
}

impl Base16 {
  fn chunk_styles(&self, base: Style) -> BTreeMap<ChunkKind, Style> {
    btreemap!{
      ChunkKind::Date => base.fg(self.base03),
      ChunkKind::Time => base.fg(self.base03),
      ChunkKind::FieldKey => base.fg(self.base0C),
      ChunkKind::Context => base.fg(self.base03),

      ChunkKind::Level(LogLevel::Debug) => base.fg(self.base0C),
      ChunkKind::Level(LogLevel::Info) => base.fg(self.base0B),
      ChunkKind::Level(LogLevel::Warning) => base.fg(self.base0A),
      ChunkKind::Level(LogLevel::Error) => base.fg(self.base09),
      ChunkKind::Level(LogLevel::Fatal) => base.fg(self.base08),
      ChunkKind::Level(LogLevel::Plain) => base,
      ChunkKind::Level(LogLevel::Int) => base.fg(self.base0F)
    }
  }

  pub fn to_profile_normal(&self) -> StyleProfile {
    let base = Style::new().fg(self.base05);
    StyleProfile {
      base_style: base,
      opaque: false, // TODO: make opaque configurable?
      chunk_styles: self.chunk_styles(base)
    }
  }

  pub fn to_profile_selected(&self) -> StyleProfile {
    let base = Style::new().fg(self.base05).on(self.base02);
    StyleProfile {
      base_style: base,
      opaque: true,
      chunk_styles: self.chunk_styles(base)
    }
  }

  pub fn to_profile_highlighted(&self) -> StyleProfile {
    let base = Style::new().fg(self.base06).bold();
    StyleProfile {
      base_style: base,
      opaque: false,
      chunk_styles: self.chunk_styles(base)
    }
  }
}

pub struct StyleProfile {
  base_style: Style,
  opaque: bool,

  chunk_styles: BTreeMap<ChunkKind, Style>
}

impl fmt::Debug for StyleProfile {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "StyleProfile {{ ")?;
    write!(f, "{}", self.base_style.paint("base"))?;

    for (i, (kind, style)) in self.chunk_styles.iter().enumerate() {
      if i == 0 {
        write!(f, ",")?;
      }

      write!(f, " {}", style.paint(kind.to_string()))?;

      if i < self.chunk_styles.len() - 1 {
        write!(f, ",")?;
      }
    }
    
    write!(f, " }}")?;

    Ok(())
  }
}

impl StyleProfile {
  pub fn plain() -> StyleProfile {
    StyleProfile {
      base_style: Style::new(),
      opaque: false,
      chunk_styles: btreemap! {}
    }
  }

  pub fn default_normal() -> StyleProfile {
    let base = Style::new();

    StyleProfile {
      base_style: base,
      opaque: false,
      chunk_styles: btreemap!{
        ChunkKind::Date => base.fg(Color::White).dimmed(),
        ChunkKind::Time => base.fg(Color::White).dimmed(),
        ChunkKind::FieldKey => base.fg(Color::Cyan).dimmed(),
        ChunkKind::Context => base.fg(Color::Black).bold(),

        ChunkKind::Level(LogLevel::Debug) => base.fg(Color::Cyan),
        ChunkKind::Level(LogLevel::Info) => base.fg(Color::Green),
        ChunkKind::Level(LogLevel::Warning) => base.fg(Color::Yellow),
        ChunkKind::Level(LogLevel::Error) => base.fg(Color::Red),
        ChunkKind::Level(LogLevel::Fatal) => base.fg(Color::Red).bold(),
        ChunkKind::Level(LogLevel::Plain) => base,
        ChunkKind::Level(LogLevel::Int) => base.fg(Color::Purple).bold()
      }
    }
  }

  pub fn default_selected() -> StyleProfile {
    let base = Style::new().on(Color::White).fg(Color::Black);

    StyleProfile {
      base_style: base,
      opaque: true,
      chunk_styles: btreemap!{
        ChunkKind::FieldKey => base.fg(Color::Blue).dimmed(),

        ChunkKind::Level(LogLevel::Debug) => base.fg(Color::Blue),
        ChunkKind::Level(LogLevel::Info) => base.fg(Color::Green).dimmed(),
        ChunkKind::Level(LogLevel::Warning) => base.fg(Color::Purple).dimmed(),
        ChunkKind::Level(LogLevel::Error) => base.fg(Color::Red).dimmed(),
        ChunkKind::Level(LogLevel::Fatal) => base.fg(Color::Red).dimmed().bold(),
        ChunkKind::Level(LogLevel::Int) => base.fg(Color::Purple).bold()
      }
    }
  }

  pub fn default_highlighted() -> StyleProfile {
    let base = Style::new().bold();

    StyleProfile {
      base_style: base,
      opaque: false,
      chunk_styles: btreemap!{
        ChunkKind::Date => base.fg(Color::White).dimmed(),
        ChunkKind::Time => base.fg(Color::White).dimmed(),
        ChunkKind::FieldKey => base.fg(Color::Cyan).dimmed(),
        ChunkKind::Context => base.fg(Color::Black),

        ChunkKind::Level(LogLevel::Debug) => base.fg(Color::Cyan),
        ChunkKind::Level(LogLevel::Info) => base.fg(Color::Green),
        ChunkKind::Level(LogLevel::Warning) => base.fg(Color::Yellow),
        ChunkKind::Level(LogLevel::Error) => base.fg(Color::Red),
        ChunkKind::Level(LogLevel::Fatal) => base.fg(Color::Red),
        ChunkKind::Level(LogLevel::Plain) => base,
        ChunkKind::Level(LogLevel::Int) => base.fg(Color::Purple)
      }
    }
  }

  pub fn is_opaque(&self) -> bool {
    self.opaque
  }

  pub fn get_base(&self) -> &Style {
    &self.base_style
  }

  pub fn get_style(&self, kind: &ChunkKind) -> &Style {
    match self.chunk_styles.get(kind) {
      Some(chunk_style) => chunk_style,
      None => &self.base_style
    }
  }
}

#[derive(Copy, Clone)]
pub enum StyleProfileKind {
  //Normal,
  Selected,
  //Highlighted
}

#[derive(Debug)]
pub struct StyleConfig {
  pub normal: StyleProfile,
  pub selected: StyleProfile,
  pub highlighted: StyleProfile
}

impl StyleConfig {
  pub fn default() -> Self {
    StyleConfig {
      normal: StyleProfile::default_normal(),
      selected: StyleProfile::default_selected(),
      highlighted: StyleProfile::default_highlighted()
    }
  }

  pub fn from_base16(base16: &Base16) -> Self {
    StyleConfig {
      normal: base16.to_profile_normal(),
      selected: base16.to_profile_selected(),
      highlighted: base16.to_profile_highlighted()
    }
  }

  pub fn get_profile(&self, kind: StyleProfileKind) -> &StyleProfile {
    match kind {
      //StyleProfileKind::Normal => &self.normal,
      StyleProfileKind::Selected => &self.selected,
      //StyleProfileKind::Highlighted => &self.highlighted
    }
  }
}

fn load_base16(path: &str) -> Result<StyleConfig, Box<dyn Error>> {
  let expanded_path = shellexpand::full(path)?;
  let file = File::open(&expanded_path.to_string())?;
  let reader = BufReader::new(file);

  let b16: Base16 = serde_yaml::from_reader(reader)?;
  Ok(StyleConfig::from_base16(&b16))
}

impl FromStr for StyleConfig {
  type Err = Box<dyn Error>;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    lazy_static! {
      static ref RE: Regex = Regex::new(r"^(?:base16|b16)[:=](\S+)$").unwrap();
    }

    if let Some(groups) = RE.captures(s) {
      if let Some(path) = groups.get(1) {
        load_base16(path.as_str())
      } else {
        bail!(format!("invalid b16: {}", s))
      }
    } else {
      match s {
        "default" => Ok(StyleConfig::default()),
        _ => bail!(format!("unsupported style profile: {}", s))
      }
    }
  }
}

pub type Styler = Box<Rc<dyn Fn(&StyleConfig) -> Style>>;

pub fn styler_base(kind: StyleProfileKind) -> Styler {
  Box::new(Rc::new(move |c| *c.get_profile(kind).get_base()))
}

pub fn styler_error(kind: StyleProfileKind) -> Styler {
  Box::new(Rc::new(move |c| {
    *c.get_profile(kind).get_style(&ChunkKind::Level(LogLevel::Error))
  }))
}
