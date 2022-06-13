//! Socket and pipe helpers.

use std::{
    net::SocketAddr,
    os::unix::prelude::OsStrExt,
    path::{Path, PathBuf},
};

use tokio::io::{AsyncRead, AsyncWrite};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

use crate::error::Error;

/// Abstraction for a client connection.
///
/// Implementations should automatically handle temporary OS errors such as
/// errors during a brief moment the server is busy accepting a client.
///
/// Note: This is a `async_trait`.
#[async_trait::async_trait]
pub trait Connect<S>
where
    S: AsyncRead + AsyncWrite,
{
    /// Connect to the service and return a stream.
    async fn connect(&self) -> Result<S, Error>;
}

/// Configures and creates a stream to a local service.
///
/// The underlying connection is either a Unix domain socket or a Windows named pipe.
pub struct LocalConnector {
    session_id: String,
    service_id: String,
}

impl LocalConnector {
    pub fn new() -> Self {
        Self {
            session_id: default_session_id(),
            service_id: String::new(),
        }
    }

    /// Set the name of the service to connect to.
    pub fn with_service_id<S: Into<String>>(mut self, service_id: S) -> Self {
        self.service_id = service_id.into();
        self
    }
}

#[cfg(unix)]
#[async_trait::async_trait]
impl Connect<UnixStream> for LocalConnector {
    async fn connect(&self) -> Result<UnixStream, Error> {
        let path = get_unix_socket_path(&self.session_id, &self.service_id);
        let stream = UnixStream::connect(path).await?;

        Ok(stream)
    }
}

#[cfg(windows)]
#[async_trait::async_trait]
impl Connect<NamedPipeClient> for LocalConnector {
    async fn connect(&self) -> Result<NamedPipeClient, Error> {
        let path = get_windows_named_pipe_path(&self.session_id, &self.service_id);

        loop {
            match ClientOptions::new().open(path) {
                Ok(client) => return Ok(client),
                Err(e) if e.raw_os_error() == Some(winerror::ERROR_PIPE_BUSY as i32) => (),
                Err(e) => return Err(e),
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

impl Default for LocalConnector {
    fn default() -> Self {
        Self::new()
    }
}

/// Abstraction for a server connection.
///
/// Implementations should automatically handle temporary OS errors such as
/// an error accepting a connection because it is already closed.
///
/// Note: This is a `async_trait`.
#[async_trait::async_trait]
pub trait Listen<S>
where
    S: AsyncRead + AsyncWrite,
{
    /// Begin listening for client connections to the host.
    fn listen(&mut self) -> Result<Option<SocketAddr>, Error>;

    /// Wait for a client connection.
    async fn accept(&mut self) -> Result<(S, Option<SocketAddr>), Error>;
}

/// Configures and creates a stream for a local service.
///
/// The underlying connection is either a Unix domain socket or a Windows named pipe.
pub struct LocalListener {
    session_id: String,
    service_id: String,

    #[cfg(unix)]
    listener: Option<UnixListener>,
    #[cfg(unix)]
    path: Option<PathBuf>,

    #[cfg(windows)]
    server: Option<NamedPipeServer>,
}

impl LocalListener {
    pub fn new() -> Self {
        Self {
            session_id: default_session_id(),
            service_id: String::new(),

            #[cfg(unix)]
            listener: None,
            #[cfg(unix)]
            path: None,

            #[cfg(windows)]
            server: None,
        }
    }

    /// Set the name of the service.
    pub fn with_service_id<S: Into<String>>(mut self, service_id: S) -> Self {
        self.service_id = service_id.into();
        self
    }
}

#[cfg(unix)]
#[async_trait::async_trait]
impl Listen<UnixStream> for LocalListener {
    fn listen(&mut self) -> Result<Option<SocketAddr>, Error> {
        let path = get_unix_socket_path(&self.session_id, &self.service_id);

        // Existing file causes address in use error.
        // TODO: Check if file is actually stale.
        let _ = std::fs::remove_file(&path);

        let listener = UnixListener::bind(&path)?;

        self.listener = Some(listener);
        self.path = Some(path);

        Ok(None)
    }

    async fn accept(&mut self) -> Result<(UnixStream, Option<SocketAddr>), Error> {
        loop {
            match self.listener.as_ref().unwrap().accept().await {
                Ok((stream, _addr)) => return Ok((stream, None)),
                Err(error) if is_fatal_accept(&error) => return Err(error.into()),
                _ => continue,
            }
        }
    }
}

#[cfg(unix)]
impl Drop for LocalListener {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(windows)]
#[async_trait::async_trait]
impl Listen<NamedPipeServer> for LocalListener {
    fn listen(&mut self) -> Result<Option<SocketAddr>, Error> {
        let path = get_windows_named_pipe_path(&self.session_id, &self.service_id);
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(path)?;

        self.server = Some(server);

        Ok(None)
    }

    async fn accept(&mut self) -> Result<(NamedPipeServer, Option<SocketAddr>), Error> {
        let path = get_windows_named_pipe_path(&self.session_id, &self.service_id);
        let mut server = self.server.take().unwrap();

        // Accept a client and immediately start a new server to minimize downtime.
        server.connect().await?;
        let new_server = ServerOptions::new().create(path)?;

        self.server = Some(new_server);

        Ok((server, None))
    }
}

impl Default for LocalListener {
    fn default() -> Self {
        Self::new()
    }
}

fn default_session_id() -> String {
    match std::env::current_dir() {
        Ok(path) => path_to_session_id(&path),
        Err(_) => whoami::username(),
    }
}

fn path_to_session_id(path: &Path) -> String {
    let hash = mx3::hash(path.as_os_str().as_bytes(), 1);

    format!("{:016x}", hash)
}

fn get_runtime_dir() -> PathBuf {
    let mut runtime_dir = dirs::runtime_dir();

    if runtime_dir.is_none() {
        runtime_dir = Some(std::env::temp_dir());
    }

    runtime_dir.unwrap()
}

fn get_filename(session_id: &str, service_id: &str) -> String {
    let username = whoami::username();
    let username =
        percent_encoding::utf8_percent_encode(&username, percent_encoding::NON_ALPHANUMERIC);
    let session_id =
        percent_encoding::utf8_percent_encode(session_id, percent_encoding::NON_ALPHANUMERIC);
    let service_id =
        percent_encoding::utf8_percent_encode(service_id, percent_encoding::NON_ALPHANUMERIC);

    format!("webaves-{}-{}-{}", username, session_id, service_id)
}

#[cfg(unix)]
fn get_unix_socket_path(session_id: &str, service_id: &str) -> PathBuf {
    let mut path = get_runtime_dir();

    path.push(get_filename(session_id, service_id));
    path.set_extension("sock");

    path
}

#[cfg(windows)]
fn get_windows_named_pipe_path(session_id: &str, service_id: &str) -> PathBuf {
    let mut path = PathBuf::from(r"\\.\pipe\");

    path.push(get_filename(session_id, service_id));

    path
}

fn is_fatal_accept(error: &std::io::Error) -> bool {
    !matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::BrokenPipe
    )
}
