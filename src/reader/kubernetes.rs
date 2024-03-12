// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::prelude::*;
use rand::prelude::*;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use simple_error::{SimpleError, SimpleResult};
use subprocess::{Popen, PopenConfig, Redirection, Exec};

use crate::config::Config;
use crate::renderer::LogEntry;
use crate::parser::ReaderMetadata;
use crate::parser::util::normalize_datetime;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct Container {
  namespace: String,
  pod: String,
  container: String,
  siblings: usize
}

impl Container {
  pub fn new(
    namespace: String, pod: String, container: String, siblings: usize
  ) -> Self {
    Container { namespace, pod, container, siblings }
  }
}

impl fmt::Display for Container {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    if self.siblings > 2 {
      fmt.write_str(&self.pod)?;
      fmt.write_str("/")?;
      fmt.write_str(&self.container)?;
    } else {
      fmt.write_str(&self.pod)?;
    }

    Ok(())
  }
}

#[derive(Debug)]
enum PodEvent {
  Added(Container),
  Removed(Container)
}

#[derive(Debug, Deserialize)]
struct KubernetesMetadata {
  name: String,
  namespace: String,
  labels: HashMap<String, String>
}

#[derive(Debug, Deserialize)]
struct KubernetesContainer {
  name: String
}

#[derive(Debug, Deserialize)]
struct KubernetesPodSpec {
  containers: Vec<KubernetesContainer>
}

#[derive(Debug, Deserialize)]
enum KubernetesPodPhase {
  Pending,
  Running,
  Succeeded,
  Failed,
  Unknown
}

#[derive(Debug, Deserialize)]
struct KubernetesContainerStateWaiting {}

#[derive(Debug, Deserialize)]
struct KubernetesContainerStateRunning {}

#[derive(Debug, Deserialize)]
struct KubernetesContainerStateTerminated {
  #[serde(rename = "exitCode")]
  exit_code: isize
}

#[derive(Debug, Deserialize)]
struct KubernetesContainerState {
  waiting: Option<KubernetesContainerStateWaiting>,
  running: Option<KubernetesContainerStateRunning>,
  terminated: Option<KubernetesContainerStateTerminated>
}

#[derive(Debug, Deserialize)]
struct KubernetesContainerStatus {
  name: String,
  ready: bool,

  #[serde(rename = "restartCount")]
  restart_count: isize,

  state: KubernetesContainerState,

  #[serde(rename = "lastState")]
  last_state: KubernetesContainerState
}

#[derive(Debug, Deserialize)]
struct KubernetesPodStatus {
  phase: KubernetesPodPhase,

  #[serde(rename = "containerStatuses")]
  container_statuses: Vec<KubernetesContainerStatus>
}

#[derive(Debug, Deserialize)]
struct KubernetesPod {
  metadata: KubernetesMetadata,
  spec: KubernetesPodSpec,
  status: KubernetesPodStatus
}

#[derive(Debug, Deserialize)]
struct KubernetesListObject {
  items: Vec<KubernetesPod>
}

fn get_containers(pod: &KubernetesPod) -> Vec<Container> {
  let mut ret = Vec::new();

  let pod_name = pod.metadata.name.clone();
  let siblings = pod.spec.containers.len();
  for container in &pod.spec.containers {
    ret.push(Container::new(
      pod.metadata.namespace.clone(),
      pod_name.clone(),
      container.name.clone(),
      siblings
    ));
  }

  ret
}

/// determines if the args are a kubernetes labelSelector or a simple pod name
/// selector
fn is_selector<T: AsRef<str>>(args: &[T]) -> bool {
  if args.is_empty() || args.len() > 1 {
    return false;
  }

  let sel = &args[0].as_ref();

  // try to check for as many selector characters as possible in 1 iter
  for c in sel.chars() {
    match c {
      '=' | '!' | '(' | ')' => return true,
      _ => ()
    }
  }

  sel.contains(" in ") || sel.contains(" notin ")
}

// determines if a pod matches at least one simple selector argument (i.e. that
// the name contains the arg as a substring)
fn pod_matches<T: AsRef<str>>(pod: &KubernetesPod, args: &[T]) -> bool {
  if args.is_empty() {
    return true;
  }

  for arg in args {
    if pod.metadata.name.contains(arg.as_ref()) {
      return true;
    }
  }

  false
}

fn wrap_watch(
  config: Arc<Config>,
  namespace: String, port: u16,
  log_tx: Sender<LogEntry>,
  event_tx: Sender<PodEvent>,
) -> SimpleResult<()> {
  let use_selector = is_selector(&config.app);
  let query = if use_selector {
    let selector = &config.app[0];

    log_tx.send(LogEntry::internal(&format!(
      "watching pods matching {} in namespace {}",
      &selector, &namespace
    ))).ok();

    vec![("labelSelector".to_string(), selector.clone())]
  } else if config.app.is_empty() {
    log_tx.send(LogEntry::internal(
      &format!("watching namespace {}", &namespace)
    )).ok();

    vec![]
  } else {
    let names = config.app.iter()
      .map(|arg| format!("{:?}", arg))
      .collect::<Vec<String>>()
      .join(", ");

    log_tx.send(LogEntry::internal(&format!(
      "watching pods in namespace {} containing: {}",
       &namespace, names
    ))).ok();

    vec![]
  };

  let mut current_containers: HashSet<Container> = HashSet::new();

  // unfortunately watch is prone to timeouts, especially if behind a proxy
  // so we'll have to poll instead :(
  let client = Client::new();
  loop {
    let mut response = client
      .get(&format!(
        "http://localhost:{port}/api/v1/namespaces/{namespace}/pods",
        port = port, namespace = namespace
      ))
      .query(&query)
      .send().map_err(SimpleError::from)?;

    if !response.status().is_success() {
      return Err(SimpleError::new("failed to list pods in namespace"))
    }

    let pod_list: KubernetesListObject = response.json()
      .map_err(SimpleError::from)?;

    let new_containers: HashSet<Container> = pod_list.items.iter()
      .filter(|pod| use_selector || pod_matches(pod, &config.app))
      .map(|pod| get_containers(pod))
      .flatten()
      .collect();

    let added = new_containers.difference(&current_containers);
    for container in added {
      event_tx.send(
        PodEvent::Added(container.clone())
      ).map_err(SimpleError::from)?;
    }

    let removed = current_containers.difference(&new_containers);
    for container in removed {
      event_tx.send(
        PodEvent::Removed(container.clone())
      ).map_err(SimpleError::from)?;
    }

    current_containers = new_containers;

    thread::sleep(Duration::from_secs(config.kubernetes.poll_interval));
  }
}

fn watch_events(
  config: Arc<Config>,
  namespace: String, port: u16,
  log_tx: Sender<LogEntry>,
  event_tx: Sender<PodEvent>
) -> JoinHandle<SimpleResult<()>> {
  thread::spawn(move || {
    match wrap_watch(config, namespace, port, log_tx.clone(), event_tx) {
      Ok(()) => (),
      Err(e) => {
        log_tx.send(LogEntry::internal(&format!(
          "watch ended with error: {:?}", e
        ))).ok();

        // not technically eof as some individual log follows may still be
        // working, but close enough - eof is just informational
        log_tx.send(LogEntry::eof()).ok();
        eprintln!("watch exited with error: {:?}", e)
      }
    };

    Ok(())
  })
}

/// Attempts to retrieve the current status of the given container
///
/// If the container no longer exists, returns `Ok(None)`, otherwise returns
/// `Ok(Some(status))`
fn get_container_status(
  namespace: &str, port: u16,
  container: &Container
) -> SimpleResult<Option<KubernetesContainerStatus>> {
  let client = Client::new();
  let mut response = client
    .get(&format!(
      "http://localhost:{port}/api/v1/namespaces/{namespace}/pods/{pod}",
      port = port, namespace = namespace, pod = &container.pod
    ))
    .send()
    .map_err(SimpleError::from)?;

  let http_status = response.status();
  if http_status == StatusCode::NOT_FOUND {
    return Ok(None);
  } else if !response.status().is_success() {
    return Err(SimpleError::new(format!(
      "unable to get pod status: {}",
      response.status().as_u16()
    )));
  }

  let pod: KubernetesPod = response.json().map_err(SimpleError::from)?;
  let status = pod.status.container_statuses.into_iter()
    .find(|c| c.name == container.container);

  Ok(status)
}

fn should_stop_following(
  namespace: &str, port: u16,
  container: &Container,
  tx: Sender<LogEntry>
) -> bool {
  match get_container_status(&namespace, port, &container) {
    Ok(Some(status)) => {
      if status.state.running.is_some() {
        // log ran out, but the container is still running
        // either it restarted already or there was a network issue
        tx.send(LogEntry::internal(&format!(
          "container log was interrupted: {}", container
        ))).ok();

        false
      } else if let Some(terminated) = status.state.terminated {
        tx.send(LogEntry::internal(&format!(
          "container {} terminated with code {}",
          container, terminated.exit_code)
        )).ok();

        true
      } else {
        false
      }
    },
    Ok(None) => {
      tx.send(LogEntry::internal(&format!(
        "container {} has been removed", container)
      )).ok();

      true
    },
    Err(e) => {
      tx.send(LogEntry::internal(&format!(
        "error watching container {}: {}", container, e.to_string()
      ))).ok();

      true
    }
  }
}

fn parse_line<'a>(
  line: &'a str
) -> SimpleResult<(DateTime<Utc>, &'a str)> {
  let mut splits = line.splitn(2, ' ');

  let dt_fixed = DateTime::parse_from_rfc3339(splits.next().unwrap())
    .map_err(SimpleError::from)?;

  let dt_utc = normalize_datetime(
    &dt_fixed.naive_local(), Some(dt_fixed.timezone())
  );

  let rest = splits.next()
    .ok_or_else(|| SimpleError::new("could not parse line"))?;

  Ok((dt_utc, rest))
}

fn follow_log(
  config: Arc<Config>,
  port: u16,
  container: Container,
  tx: Sender<LogEntry>
) {
  thread::spawn(move || {
    let client = Client::new();

    // a count of retry attempts
    // this value may be reset if the log successfully runs for long enough
    let mut retries = 0;

    // TODO: save last timestamp
    // if the log is interrupted, we can avoid duplicating messages
    // TODO: should query latest pod status to see if it's terminating

    loop {
      if retries > 2 {
        tx.send(LogEntry::internal(
          &format!("giving up watching container due to errors: {}", container)
        )).ok();

        break;
      } else if retries > 0 {
        // if this is the 2nd (or nth) try, wait a bit
        // maybe the pod wasn't ready?
        thread::sleep(Duration::from_millis(5000));
      }

      // check to make sure the container still exists
      if should_stop_following(&container.namespace, port, &container, tx.clone()) {
        break;
      }

      tx.send(LogEntry::internal(&format!(
        "started watching container: {}", container
      ))).ok();

      let query = vec![
        ("follow", "true"),
        ("container", &container.container),
        ("timestamps", "true")
      ];

      let maybe_response = client
        .get(&format!(
          "http://localhost:{port}/api/v1/namespaces/{namespace}/pods/{pod}/log",
          port = port, namespace = container.namespace, pod = &container.pod
        ))
        .query(&query)
        .send();

      let response = match maybe_response {
        Ok(response) => response,
        Err(e) => {
          tx.send(LogEntry::internal(
            &format!("error watching container {}: {:?}", container, e)
          )).ok();

          retries += 1;
          continue;
        }
      };

      // todo: could try to parse out the error message field but lazy
      if !response.status().is_success() {
        tx.send(LogEntry::internal(
          &format!("error watching container {}", container)
        )).ok();

        retries += 1;
        continue;
      }

      let reader = BufReader::new(response);
      for (i, line) in reader.lines().enumerate() {
        // skip bad lines
        let line = match line {
          Ok(line) => line,
          Err(_) => continue
        };

        let mut timestamp = None;
        let parsed = match parse_line(&line) {
          Ok((ts, line)) => {
            timestamp = Some(ts);

            line
          },
          Err(_) => &line
        };

        let meta = ReaderMetadata {
          timestamp,
          source: Some(container.to_string())
        };

        // TODO: need some special parsing magic
        // need container name available, and we can fill dates using timestamps=true
        // can we pass this info in directly as pre-parsed chunks?
        match LogEntry::message(Arc::clone(&config), parsed, Some(meta)) {
          Ok(Some(entry)) => tx.send(entry).ok(),
          _ => continue
        };

        // assume the error state clears as long as we read a couple lines
        // this allows "Unable to retrieve container logs..." messages to count
        // toward the retry limit
        if i > 1 {
          retries = 0;
        }
      }

      retries += 1;

      // wait a bit ... apparently kubernetes still considers a pod running for
      // a brief period after it sends an EOF
      thread::sleep(Duration::from_millis(500));

      // decide if we should restart the log
      if should_stop_following(&container.namespace, port, &container, tx.clone()) {
        break;
      }
    }
  });
}

/// spawns a kubectl proxy, returning a port and a handle for the child process
///
/// the port will be randomly selected; if kubectl exits quickly (for example,
/// due to a port conflict), an Err is returned.
fn spawn_kubectl(config: Arc<Config>) -> SimpleResult<(Popen, u16)> {
  let port = if let Some(port) = config.kubernetes.port {
    port
  } else {
    thread_rng().gen_range(1000, 65535)
  };

  let port_arg = &format!("--port={}", port);
  let args = vec![
    "kubectl",
    "proxy",
    &port_arg
  ];

  let mut child = Popen::create(&args, PopenConfig {
    stdout: Redirection::Merge,
    stderr: Redirection::None,

    ..Default::default()
  }).map_err(SimpleError::from)?;

  // wait a bit to see if it exits
  thread::sleep(Duration::from_millis(250));

  if child.poll().is_some() {
    Err(SimpleError::new("kubectl exited early"))
  } else {
    Ok((child, port))
  }
}

fn kubectl_get_namespace() -> SimpleResult<String> {
  // kubectl _appears_ to helpfully rewrite the config output to show the
  // current context first... but since that may or may not be intended
  // behavior, we also pass --minify which removes all but the current context
  let data = Exec::cmd("kubectl")
    .args(&[
      "config",
      "view",
      "--minify",
      "-o",
      "jsonpath={.contexts[0].context.namespace}"
    ])
    .stdout(Redirection::Pipe)
    .stderr(Redirection::Pipe)
    .capture()
    .map_err(SimpleError::from)?;

  if data.success() {
    let output = data.stdout_str();
    if output.is_empty() {
      // namespace is unset, assume default
      Ok("default".to_string())
    } else {
      Ok(output)
    }
  } else {
    Err(SimpleError::new(format!(
      "kubectl error: {}", data.stderr_str()
    )))
  }
}

fn list_namespaces(config: &Config) -> SimpleResult<Vec<String>> {
  if !config.kubernetes.namespaces.is_empty() {
    return Ok(config.kubernetes.namespaces.clone())
  } else {
    return Ok(vec![kubectl_get_namespace()?])
  }
}

pub fn read_kubernetes_selector(
  config: Arc<Config>,
  tx: Sender<LogEntry>,
  exit_req_rx: Receiver<()>,
  exit_resp_tx: Sender<()>
) -> JoinHandle<SimpleResult<()>> {
  thread::Builder::new().name("read_kubernetes_selector".to_string()).spawn(move || {
    let (mut kubectl, port) = spawn_kubectl(Arc::clone(&config))?;
    tx.send(LogEntry::internal(
      &format!("started kubernetes api proxy on port {}", port)
    )).ok();

    let (event_tx, event_rx) = channel();
    for namespace in list_namespaces(&config)? {
      watch_events(
        Arc::clone(&config), namespace.clone(), port, tx.clone(), event_tx.clone()
      );
    }

    loop {
      thread::sleep(Duration::from_millis(100));

      if let Ok(()) = exit_req_rx.try_recv() {
        break;
      }

      for event in event_rx.try_iter() {
        match event {
          PodEvent::Added(container) => {
            follow_log(
              Arc::clone(&config),
              port,
              container,
              tx.clone()
            );
          },
          PodEvent::Removed(_container) => {
            // TODO: do we care?
          }
        }
      }
    }

    kubectl.terminate().ok();
    kubectl.wait().ok();

    exit_resp_tx.send(()).ok();

    Ok(())
  }).unwrap()
}
