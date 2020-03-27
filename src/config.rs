// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use std::sync::Arc;

use atty::{self, Stream};
use regex::Regex;
use serde::Deserialize;
use serde::de::{self, Visitor, Deserializer};
use shellexpand;
use simple_error::SimpleError;
use structopt::StructOpt;

use crate::style::StyleConfig;
use crate::reader;
use crate::renderer;

#[derive(Debug)]
pub enum RendererType {
  Auto,
  Plain,
  Raw,
  Json,
  Styled,
  Interactive
}

fn get_auto_renderer(config: Arc<Config>) -> renderer::Renderer {
  // probably best not to infinitely loop
  let preferred = match config.preferred_renderer {
    RendererType::Auto => renderer::interactive_renderer,
    _ => {
      config.preferred_renderer.get_renderer(Arc::clone(&config))
    }
  };

  if atty::is(Stream::Stdout) {
    preferred
  } else {
    renderer::plain_renderer
  }
}

impl RendererType {
  pub fn get_renderer(&self, config: Arc<Config>) -> renderer::Renderer {
    match *self {
      RendererType::Auto => get_auto_renderer(config),
      RendererType::Plain => renderer::plain_renderer,
      RendererType::Raw => renderer::raw_renderer,
      RendererType::Json => renderer::json_renderer,
      RendererType::Styled => renderer::styled_renderer,
      RendererType::Interactive => renderer::interactive_renderer,
    }
  }
}

impl FromStr for RendererType {
  type Err = Box<dyn Error>;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "auto" => Ok(RendererType::Auto),
      "plain" => Ok(RendererType::Plain),
      "raw" => Ok(RendererType::Raw),
      "json" => Ok(RendererType::Json),
      "styled" => Ok(RendererType::Styled),
      "interactive" => Ok(RendererType::Interactive),
      _ => bail!(format!("invalid renderer type: {}", s))
    }
  } 
}

fn get_auto_reader(config: Arc<Config>) -> reader::Reader {
  // TODO: is it possible to tell if stdin has some input?
  // TODO: consider detecting if k8s based on args and kubernetes::is_selector?
  if !atty::is(Stream::Stdin) {
    // /dev/stdin should exist on all unixes
    if cfg!(unix) {
      return reader::read_stdin_hack
    } else {
      return reader::read_stdin
    }
  }

  if config.kubernetes.namespace.is_some() {
    return reader::read_kubernetes_selector;
  }

  reader::read_null
}

#[derive(Debug)]
pub enum ReaderType {
  Auto,
  Stdin,
  Hack,
  Kubernetes,
  Null
  //Subprocess
}

impl ReaderType {
  pub fn get_reader(&self, config: Arc<Config>) -> reader::Reader {
    match *self {
      ReaderType::Auto => get_auto_reader(config),
      ReaderType::Stdin => reader::read_stdin,
      ReaderType::Hack => reader::read_stdin_hack,
      ReaderType::Kubernetes => reader::read_kubernetes_selector,
      ReaderType::Null => reader::read_null
      //ReaderType::Subprocess => ...
    }
  }
}

impl FromStr for ReaderType {
  type Err = Box<dyn Error>;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "auto" => Ok(ReaderType::Auto),
      "stdin" => Ok(ReaderType::Stdin),
      "hack" => Ok(ReaderType::Hack),
      "kubernetes" | "k8s" => Ok(ReaderType::Kubernetes),
      "null" => Ok(ReaderType::Null),
      _ => bail!(format!("invalid reader type: {}", s))
    }
  }
}

/// Kubernetes-specific config
#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct KubernetesConfig {
  /// kubectl path override
  /// 
  /// Path to kubectl. If unset, searches $PATH.
  #[structopt(long, short = "k", env = "WD_KUBECTL")]
  pub kubectl: Option<String>,

  /// Kubernetes namespace to use read
  #[structopt(long, short = "n", env = "WD_NAMESPACE")]
  pub namespace: Option<String>,

  /// Local kubernetes proxy port
  /// 
  /// A kubernetes API proxy will be spawned on this port over the loopback
  /// interface. If unset, a random port will be selected.
  #[structopt(long, short = "p", env = "WD_K8S_PORT")]
  pub port: Option<u16>,

  /// Poll interval while watching Kubernetes pods in seconds
  #[structopt(long, env = "WD_K8S_POLL_INTERVAL", default_value = "5")]
  pub poll_interval: u64
}

struct RegexFromStr;

impl<'de> Visitor<'de> for RegexFromStr {
  type Value = Regex;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("a string containing a valid regular expression")
  }

  fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
  where
    E: de::Error
  {
    match Regex::new(s) {
      Ok(r) => Ok(r),
      Err(e) => Err(de::Error::custom(format!(
        "could not compile regex: {:?}", e
      )))
    }
  }
}

fn de_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
  D: Deserializer<'de>
{
  deserializer.deserialize_str(RegexFromStr)
}

#[derive(Debug, Deserialize)]
pub struct RegexMapping {
  /// a Regex pattern to parse an incoming line
  #[serde(deserialize_with = "de_regex")]
  pub pattern: Regex,

  /// A Chrono datetime format string, will be applied to the `datetime` capture
  /// group
  pub datetime: Option<String>,

  /// An optional Chrono strftime string used to prepend missing fields to the
  /// timestamp before parsing
  ///
  /// Chrono isn't able to parse datetimes with missing fields (e.g. year), but
  /// some log formats (e.g. klog) leave certain fields out. This allows these
  /// formats to be parsed anyway.
  pub datetime_prepend: Option<String>
}

#[derive(Debug)]
pub struct RegexConfig {
  pub mappings: Vec<RegexMapping>
}

impl FromStr for RegexConfig {
  type Err = SimpleError;

  fn from_str(path: &str) -> Result<Self, Self::Err> {
    let expanded_path = shellexpand::full(path).map_err(SimpleError::from)?;
    let file = File::open(&expanded_path.to_string()).map_err(SimpleError::from)?;
    let reader = BufReader::new(file);

    match serde_yaml::from_reader(reader) {
      Ok(mappings) => Ok(RegexConfig { mappings }),
      Err(e) => Err(SimpleError::new(
        format!("error loading regexes {}: {:?}", path, e)
      ))
    }
  }
}

#[derive(Debug, StructOpt)]
#[structopt(
  name = "woodchipper",
  rename_all = "kebab-case",
  raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
pub struct Config {
  /// Renderer to use, one of: auto, plain, json, styled, interactive
  /// 
  /// If auto, will is determined by terminal and whether or not output will be
  /// redirected. Automatic preference may be overridden with
  /// --preferred-renderer, otherwise --renderer will force use of the given
  /// renderer even if unsupported.
  #[structopt(long, short, default_value = "auto", env = "WD_RENDERER")]
  pub renderer: RendererType,

  /// Preferred renderer, one of: plain, json, styled, interactive
  ///
  /// When --renderer=auto, this controls the preferred default renderer if no
  /// conditions exist that would otherwise select a different renderer.
  /// 
  /// For example, if you dislike the interactive renderer but still wish to
  /// automatically fall back to plaintext output when piped, use
  /// --preferred-renderer=styled.
  #[structopt(long, default_value = "interactive", env = "WD_PREFERRED_RENDERER")]
  pub preferred_renderer: RendererType,

  /// Reader to use, one of: auto, stdin, hack, kubernetes
  ///
  /// If auto, reader will be determined selected based on OS and renderer.
  ///{n}{n}
  /// - `stdin` reads from standard input{n}
  /// - `hack` reads from /dev/stdin to allow the interactive renderer to work{n}
  /// - `kubernetes` continuously follows Kubernetes pods{n}
  /// - `auto` selects `hack` on unix, unless some Kubernetes flag is set
  #[structopt(long, short = "i", default_value = "auto", env = "WD_READER")]
  pub reader: ReaderType,

  /// Kubernetes selector or subprocess args from which to capture log output.
  /// If unset, assumes logs will be read from standard input.
  pub app: Vec<String>,

  /// Fallback width for the styled renderer if no tty is detected
  ///
  /// Note that the plaintext renderer is recommended in most cases where
  /// terminal size is unavailable.
  #[structopt(
    long,
    short = "w",
    default_value = "120",
    env = "WD_FALLBACK_WIDTH"
  )]
  pub fallback_width: usize,

  /// Styled output configuration
  ///
  /// Must contain one of the following: `default`, `base16:<path to .yaml>`
  #[structopt(long, short = "s", default_value = "default", env = "WD_STYLE")]
  pub style: StyleConfig,

  /// A path to a regexes config file, which may contain custom parsing regexes
  /// for application-specific log formats.
  #[structopt(long, env = "WD_REGEXES")]
  pub regexes: Option<RegexConfig>,

  #[structopt(flatten)]
  pub kubernetes: KubernetesConfig
}
