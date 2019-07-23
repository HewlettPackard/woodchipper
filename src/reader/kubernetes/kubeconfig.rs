// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use base64;
use chrono::{DateTime, offset::Utc};
use reqwest::{
  Certificate, Client, ClientBuilder, RequestBuilder, Identity, IntoUrl,
  header::{AUTHORIZATION, HeaderValue, HeaderMap}
};
use serde::Deserialize;
use serde::de::{self, Visitor, Deserializer};
use serde_json::Value;
use snafu::{ensure, ResultExt, Snafu};
use subprocess;

#[derive(Debug, Snafu)]
pub enum Error {
  #[snafu(display(
    "unable to read kubeconfig at {}: {}",
    path.display(), source
  ))]
  ConfigRead {
    path: PathBuf,
    source: std::io::Error
  },

  #[snafu(display(
    "unable to deserialize kubeconfig at {}: {}",
    path.display(), source
  ))]
  ConfigDeserialize {
    path: PathBuf,
    source: serde_yaml::Error
  },

  #[snafu(display(
    "context missing in kubeconfig at {}: {:?}",
    path.display(), context
  ))]
  ContextNotFound {
    path: PathBuf,
    context: Option<String>
  },

  #[snafu(display(
    "could not add auth header: {}", message
  ))]
  InvalidAuthHeader {
    message: String
  },

  #[snafu(display(
    "client certificate is invalid: {}", source
  ))]
  InvalidIdentity {
    source: reqwest::Error
  },

  #[snafu(display(
    "unable to initialize reqwest client: {}", source
  ))]
  ReqwestInit {
    source: reqwest::Error
  },

  #[snafu(display(
    "error executing auth plugin {}: {}",
    command, source
  ))]
  AuthPluginExecError {
    command: String,
    source: subprocess::PopenError
  },

  #[snafu(display(
    "error from auth plugin {}: {}",
    command, message
  ))]
  AuthPluginError {
    command: String,
    message: String
  },

  #[snafu(display(
    "error deserializing result from auth plugin {}: {}",
    command, source
  ))]
  AuthPluginDeserialize {
    command: String,
    source: serde_yaml::Error
  },

  #[snafu(display(
    "error converting pem to der"
  ))]
  CertificateConversionError {
    message: String
  },

  #[snafu(display(
    "certificate could not be parsed from {}: {}",
    context, source
  ))]
  InvalidCertificate {
    context: String,
    source: reqwest::Error
  }
}

/// "Alias" for Vec<u8> to avoid puking raw cert data on Debug
#[derive(Clone)]
pub struct Bytes(Vec<u8>);

impl Deref for Bytes {
  type Target = [u8];

  fn deref(&self) -> &[u8] {
    &self.0
  }
}

impl fmt::Debug for Bytes {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "&u8[{}]", self.0.len())
  }
}

type Result<T, E = Error> = std::result::Result<T, E>;

struct BytesFromPathStr;

impl<'de> Visitor<'de> for BytesFromPathStr {
  type Value = Bytes;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("a string containing a path to an existing file")
  }

  fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
  where
    E: de::Error
  {
    match File::open(s) {
      Ok(mut file) => {
        let mut data = Vec::new();

        match file.read_to_end(&mut data) {
          Ok(_) => Ok(Bytes(data)),
          Err(e) => Err(de::Error::custom(format!(
            "error reading file at {}: {:?}", s, e
          )))
        }
      },
      Err(e) => Err(de::Error::custom(format!(
        "unable to open file at {}: {:?}", s, e
      )))
    }
  }
}

fn de_path_bytes<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
where
  D: Deserializer<'de>
{
  deserializer.deserialize_str(BytesFromPathStr)
}

struct BytesFromBase64;

impl<'de> Visitor<'de> for BytesFromBase64 {
  type Value = Bytes;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("a string containing base64 data")
  }

  fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
  where
    E: de::Error
  {
    match base64::decode(s) {
      Ok(data) => {
        Ok(Bytes(data))
      },
      Err(e) => Err(de::Error::custom(format!(
        "unable to decode base64 string: {:?}", e
      )))
    }
  }
}

fn de_base64_bytes<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
where
  D: Deserializer<'de>
{
  deserializer.deserialize_str(BytesFromBase64)
}

struct BytesFromStr;

impl<'de> Visitor<'de> for BytesFromStr {
  type Value = Bytes;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("a string containing data")
  }

  fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
  where
    E: de::Error
  {
    Ok(Bytes(s.bytes().collect()))
  }
}

fn de_str_bytes<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
where
  D: Deserializer<'de>
{
  deserializer.deserialize_str(BytesFromStr)
}

// https://github.com/vityafx/serde-aux/blob/574574cbb3d38568454707846edd2387bf4b0e48/src/field_attributes.rs#L360-L366
// (MIT)
fn de_default_from_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
  D: Deserializer<'de>,
  T: Deserialize<'de> + Default,
{
  Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterCAFile {
  #[serde(rename = "certificate-authority", deserialize_with = "de_path_bytes")]
  certificate: Bytes
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterCAEmbedded {
  #[serde(
    rename = "certificate-authority-data",
    deserialize_with = "de_base64_bytes"
  )]
  certificate: Bytes
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ClusterCA {
  File(ClusterCAFile),
  Embedded(ClusterCAEmbedded)
}

fn default_skip_tls_verify() -> bool {
  false
}

#[serde(rename_all = "kebab-case")]
#[derive(Debug, Deserialize, Clone)]
pub struct Cluster {
  server: String,

  #[serde(default = "default_skip_tls_verify")]
  insecure_skip_tls_verify: bool,

  #[serde(flatten)]
  certificate_authority: Option<ClusterCA>
}

#[derive(Debug, Deserialize)]
pub struct ClusterContainer {
  pub name: String,
  pub cluster: Cluster
}

#[derive(Debug, Deserialize)]
pub struct Context {
  pub cluster: String,
  pub namespace: Option<String>,
  pub user: String
}

#[derive(Debug)]
pub struct ResolvedContext<'a> {
  pub namespace: &'a str,
  pub auth: &'a Auth,
  pub cluster: &'a Cluster
}

#[derive(Debug, Deserialize)]
pub struct ContextContainer {
  pub name: String,
  pub context: Context
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExecAuth {
  #[serde(rename = "apiVersion")]
  pub api_version: String,
  pub command: String,

  #[serde(default)]
  pub args: Vec<String>,

  #[serde(deserialize_with = "de_default_from_null")]
  pub env: HashMap<String, String>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecCredentialToken {
  token: String,
  expiration_timestamp: Option<DateTime<Utc>>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecCredentialCertificateEmbedded {
  #[serde(rename = "clientCertificateData", deserialize_with = "de_str_bytes")]
  certificate: Bytes,

  #[serde(rename = "clientKeyData", deserialize_with = "de_str_bytes")]
  key: Bytes,

  expiration_timestamp: Option<DateTime<Utc>>
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ExecCredentialStatus {
  Token(ExecCredentialToken),
  CertificateEmbedded(ExecCredentialCertificateEmbedded)
}

impl ExecCredentialStatus {
  pub fn expiration(&self) -> Option<DateTime<Utc>> {
    match self {
      ExecCredentialStatus::CertificateEmbedded(cred) => cred.expiration_timestamp,
      ExecCredentialStatus::Token(cred) => cred.expiration_timestamp
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct ExecCredential {
  #[serde(rename = "apiVersion")]
  pub api_version: String,

  pub kind: String,
  pub status: ExecCredentialStatus
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthPlain {
  username: String,
  password: String
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthToken {
  token: String
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AuthCertificateFile {
  #[serde(rename = "client-certificate", deserialize_with = "de_path_bytes")]
  certificate: Bytes,

  #[serde(rename = "client-key", deserialize_with = "de_path_bytes")]
  key: Bytes
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AuthCertificateEmbedded {
  #[serde(
    rename = "client-certificate-data",
    deserialize_with = "de_base64_bytes"
  )]
  certificate: Bytes,

  #[serde(
    rename = "client-key-data",
    deserialize_with = "de_base64_bytes"
  )]
  key: Bytes
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthExec {
  exec: ExecAuth
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Auth {
  Plain(AuthPlain),
  Token(AuthToken),
  CertificateFile(AuthCertificateFile),
  CertificateEmbedded(AuthCertificateEmbedded),
  Exec(AuthExec),
  Null
}

impl Auth {
  /// Attempts to retrieve an ExecCredential if this is an Auth::Exec, otherwise
  /// returns Some(None)
  pub fn exec(&self) -> Result<Option<ExecCredential>> {
    let exec = if let Auth::Exec(exec) = self {
      &exec.exec
    } else {
      return Ok(None);
    };

    let env: Vec<(&str, &str)> = exec.env.iter()
      .map(|(k, v)| (k.as_str(), v.as_str()))
      .collect();

    let capture = subprocess::Exec::cmd(&exec.command)
      .args(&exec.args)
      .env_extend(&env)
      .stdout(subprocess::Redirection::Pipe)
      .stderr(subprocess::Redirection::Pipe)
      .capture()
      .context(AuthPluginExecError { command: exec.command.clone() })?;
    
    if capture.success() {
      let creds: ExecCredential = serde_yaml::from_slice(&capture.stdout)
        .context(AuthPluginDeserialize {
          command: exec.command.clone()
        })?;

      Ok(Some(creds))
    } else {
      Err(Error::AuthPluginError {
        command: exec.command.clone(),
        message: capture.stderr_str()
      })
    }
  }

  /// Attempts to create a reqwest client Identity using the configured auth,
  /// if any exists.
  pub fn identity(&self) -> Result<Option<Identity>> {
    let (cert, key) = match self {
      Auth::CertificateFile(auth) => {
        (&auth.certificate, &auth.key)
      },
      Auth::CertificateEmbedded(auth) => {
        (&auth.certificate, &auth.key)
      },
      _ => return Ok(None)
    };

    // reqwest wants these cat'd together
    let mut concat = Vec::with_capacity(cert.len() + key.len());
    concat.extend_from_slice(&cert);
    concat.extend_from_slice(&key);

    // rustls doesn't support ip address hosts
    //  - https://github.com/ctz/hyper-rustls/issues/56
    //  - https://github.com/ctz/rustls/issues/184
    //  - https://github.com/briansmith/webpki/issues/54
    //
    // also, native-tls doesn't support PEMs, or at least if it does, reqwest
    // doesn't expose that functionality
    //
    // I think we'll need to keep the kubectl subprocess workaround handy for
    // this case since it affects basically all non-cloud kubernetes apis

    Identity::from_pem(&concat).context(InvalidIdentity {}).map(Some)
  }

  pub fn token(&self) -> Option<&str> {
    match self {
      Auth::Token(auth) => {
        Some(&auth.token)
      },
      _ => None
    }
  }

  pub fn basic(&self) -> Option<String> {
    match self {
      Auth::Plain(auth) => {
        let bytes: Vec<u8> = format!("{}:{}", &auth.username, &auth.password)
          .bytes()
          .collect();

        Some(base64::encode(&bytes))
      },
      _ => None
    }
  }
}

impl Default for Auth {
  fn default() -> Self {
    Auth::Null {}
  }
}

impl From<ExecCredential> for Auth {
  fn from(exec: ExecCredential) -> Self {
    match exec.status {
      ExecCredentialStatus::Token(cred) => {
        Auth::Token(AuthToken { token: cred.token })
      },
      ExecCredentialStatus::CertificateEmbedded(cred) => {
        Auth::CertificateEmbedded(AuthCertificateEmbedded {
          certificate: cred.certificate,
          key: cred.key
        })
      }
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct User {
  #[serde(flatten, default)]
  pub auth: Auth
}

impl Default for User {
  fn default() -> Self {
    User {
      auth: Auth::Null {}
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct UserContainer {
  pub name: String,

  #[serde(default)]
  pub user: User
}

#[derive(Debug, Deserialize)]
pub struct KubernetesConfig {
  #[serde(rename = "apiVersion")]
  pub api_version: String,
  pub kind: String,

  pub clusters: Vec<ClusterContainer>,
  pub contexts: Vec<ContextContainer>,
  pub users: Vec<UserContainer>,

  #[serde(rename = "current-context")]
  pub current_context: Option<String>,

  pub preferences: HashMap<String, Value>
}

impl KubernetesConfig {
  pub fn current_context(&self) -> Option<ResolvedContext> {
    let current_context: &str = match &self.current_context {
      Some(c) => c,
      None => return None
    };

    let context = match self.contexts.iter().find(|c| c.name == current_context) {
      Some(c) => &c.context,
      None => return None
    };

    let auth = match self.users.iter().find(|u| u.name == context.user) {
      Some(u) => &u.user.auth,
      None => return None
    };

    let cluster = match self.clusters.iter().find(|c| c.name == context.cluster) {
      Some(c) => &c.cluster,
      None => return None
    };

    Some(ResolvedContext {
      auth, cluster,

      namespace: context.namespace.as_ref().map(String::as_str).unwrap_or("default")
    })
  }

  pub fn load<P>(path: P) -> Result<KubernetesConfig>
  where
    P: AsRef<Path>
  {
    let path = path.as_ref();
    let file = File::open(path).context(ConfigRead { path })?;
    let reader = BufReader::new(file);

    serde_yaml::from_reader(reader).context(ConfigDeserialize { path })
  }
}

#[derive(Debug)]
pub struct KubernetesClient {
  server: String,
  cluster: Cluster,
  auth: Auth,
  pub namespace: String,

  client: Client,

  pub auth_expiration: Option<DateTime<Utc>>
}

impl KubernetesClient {
  pub fn new(
    cluster: Cluster,
    auth: Auth,
    namespace: &str
  ) -> Result<KubernetesClient> {
    let mut builder = Client::builder()
      .use_rustls_tls()
      .use_sys_proxy();

    if cluster.insecure_skip_tls_verify {
      builder = builder.danger_accept_invalid_certs(true);
    }

    // do some basic cleanup of the server, the k8s api likes to reject calls
    // with extra slashes
    let server = cluster.server.trim_end_matches('/').to_string();

    let mut auth_expiration = None;
    let runtime_auth = if let Some(exec) = auth.exec()? {
      auth_expiration = exec.status.expiration();
      exec.into()
    } else {
      auth.clone()
    };

    let mut headers = HeaderMap::new();
    if let Some(token) = runtime_auth.token() {
      headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token)).map_err(|e| {
          Error::InvalidAuthHeader {
            message: e.to_string()
          }
        })?
      );
    } else if let Some(basic) = runtime_auth.basic() {
      headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Basic {}", basic)).map_err(|e| {
          Error::InvalidAuthHeader {
            message: e.to_string()
          }
        })?
      );
    }

    builder = builder.default_headers(headers);

    if let Some(identity) = runtime_auth.identity()? {
      builder = builder.identity(identity);
    }

    match &cluster.certificate_authority {
      Some(ClusterCA::File(ca)) => {
        let cert = Certificate::from_pem(&ca.certificate)
          .context(InvalidCertificate {
            context: "certificate-authority".to_owned()
          })?;

        builder = builder.add_root_certificate(cert);
      },
      Some(ClusterCA::Embedded(ca)) => {
        let cert = Certificate::from_pem(&ca.certificate)
          .context(InvalidCertificate {
            context: "certificate-authority-data".to_owned()
          })?;

        builder = builder.add_root_certificate(cert);
      },
      _ => ()
    };

    // initialize the client with the original auth (possibly exec) so we can
    // re-auth later if necessary (expired token, etc)
    let client = KubernetesClient {
      server, cluster, auth, auth_expiration,
      namespace: namespace.to_owned(),
      client: builder.build().context(ReqwestInit {})?
    };

    Ok(client)
  }

  /// Creates a new KubernetesClient from a ResolvedContext
  pub fn from_context(context: &ResolvedContext) -> Result<KubernetesClient> {
    KubernetesClient::new(
      context.cluster.clone(),
      context.auth.clone(),
      context.namespace
    )
  }

  /// If the current auth method has some expiration timestamp, returns true if
  /// the current credentials have expired.
  ///
  /// New credentials can be acquired using `KubernetesClient::reauth()`. At the
  /// moment, only Exec credentials can expire.
  pub fn is_expired(&self) -> bool {
    if let Some(expiration) = self.auth_expiration {
      expiration < Utc::now()
    } else {
      false
    }
  }

  /// Consumes this KubernetesClient to create a new client instance,
  /// potentially refreshing expired credentials depending on the auth type.
  ///
  /// `KubernetesClient::is_expired()` may be used to check if the current set
  /// of credentials has expired.
  pub fn reauthenticate(self) -> Result<KubernetesClient> {
    KubernetesClient::new(
      self.cluster,
      self.auth,
      &self.namespace
    )
  }

  pub fn get<S: Into<String>>(&self, path: S) -> RequestBuilder {
    self.client.get(&format!(
      "{}/{}",
      self.server, path.into().trim_start_matches('/')
    ))
  }

  pub fn post<S: Into<String>>(&self, path: S) -> RequestBuilder {
    self.client.post(&format!(
      "{}/{}",
      self.server, path.into().trim_start_matches('/')
    ))
  }
}
