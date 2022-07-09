use std::path::Path;

#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
pub use windows::*;

/// Configures and creates a client stream for local IPC.
///
/// The underlying connection is either a Unix domain socket or a Windows named pipe.
pub struct LocalConnector {
    name: String,
}

impl LocalConnector {
    /// Creates a `LocalConnector` with a given name.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self { name: name.into() }
    }
}

/// Configures and creates a server stream for local IPC.
///
/// The underlying connection is either a Unix domain socket or a Windows named pipe.
///
/// This server is intended for IPC within a user session. As such it is
/// assumed that the OS will create a socket or pipe with permissions limited
/// to the current user.
pub struct LocalListener {
    name: String,

    #[cfg(unix)]
    listener: Option<tokio::net::UnixListener>,
    #[cfg(unix)]
    path: Option<std::path::PathBuf>,
    #[cfg(windows)]
    server: Option<tokio::net::windows::named_pipe::NamedPipeServer>,
}

impl LocalListener {
    /// Creates a `LocalListener` with the given name.
    ///
    /// `name` must be a valid filename. It should be namespaced such that
    /// it does not conflict with other users or applications because the
    /// underlying socket or pipe may be globally visible (but not accessible).
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            #[cfg(unix)]
            listener: None,
            #[cfg(unix)]
            path: None,
            #[cfg(windows)]
            server: None,
        }
    }
}

#[cfg(unix)]
mod unix {
    use std::{net::SocketAddr, path::PathBuf};

    use tokio::net::{UnixListener, UnixStream};

    use crate::{
        error::Error,
        net::{Connect, Listen},
    };

    use super::{LocalConnector, LocalListener};

    #[async_trait::async_trait]
    impl Connect<UnixStream> for LocalConnector {
        async fn connect(&self) -> Result<UnixStream, Error> {
            let path = get_unix_socket_path(&self.name);
            let stream = UnixStream::connect(path).await?;

            Ok(stream)
        }
    }

    #[async_trait::async_trait]
    impl Listen<UnixStream> for LocalListener {
        fn listen(&mut self) -> Result<Option<SocketAddr>, Error> {
            let path = get_unix_socket_path(&self.name);

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

    impl Drop for LocalListener {
        fn drop(&mut self) {
            if let Some(path) = self.path.take() {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    fn get_unix_socket_path(name: &str) -> PathBuf {
        let mut path = get_runtime_dir();

        path.push(name);
        path.set_extension("sock");

        path
    }

    fn get_runtime_dir() -> PathBuf {
        let mut runtime_dir = dirs::runtime_dir();

        if runtime_dir.is_none() {
            runtime_dir = Some(std::env::temp_dir());
        }

        runtime_dir.unwrap()
    }

    fn is_fatal_accept(error: &std::io::Error) -> bool {
        !matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
        )
    }
}

#[cfg(windows)]
mod windows {
    use std::{net::SocketAddr, path::PathBuf};

    use tokio::net::windows::named_pipe::{
        ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
    };
    use winapi::shared::winerror;

    use crate::{
        error::Error,
        net::{Connect, Listen},
    };

    use super::{LocalConnector, LocalListener};

    #[async_trait::async_trait]
    impl Connect<NamedPipeClient> for LocalConnector {
        async fn connect(&self) -> Result<NamedPipeClient, Error> {
            let path = get_windows_named_pipe_path(&self.name);

            loop {
                match ClientOptions::new().open(&path) {
                    Ok(client) => return Ok(client),
                    Err(e) if e.raw_os_error() == Some(winerror::ERROR_PIPE_BUSY as i32) => (),
                    Err(e) => return Err(e.into()),
                }

                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }

    #[async_trait::async_trait]
    impl Listen<NamedPipeServer> for LocalListener {
        fn listen(&mut self) -> Result<Option<SocketAddr>, Error> {
            let path = get_windows_named_pipe_path(&self.name);
            let server = ServerOptions::new()
                .first_pipe_instance(true)
                .create(path)?;

            self.server = Some(server);

            Ok(None)
        }

        async fn accept(&mut self) -> Result<(NamedPipeServer, Option<SocketAddr>), Error> {
            let path = get_windows_named_pipe_path(&self.name);
            let server = self.server.take().unwrap();

            // Accept a client and immediately start a new server to minimize downtime.
            server.connect().await?;
            let new_server = ServerOptions::new().create(path)?;

            self.server = Some(new_server);

            Ok((server, None))
        }
    }

    fn get_windows_named_pipe_path(name: &str) -> PathBuf {
        let mut path = PathBuf::from(r"\\.\pipe\");

        path.push(name);

        path
    }
}

/// Builds a unique name that can be used for local IPC connections.
pub struct NameBuilder {
    name: String,
}

impl NameBuilder {
    /// Creates a `NameBuilder` with an empty name.
    pub fn new() -> Self {
        Self {
            name: String::new(),
        }
    }

    /// Appends the current username.
    pub fn current_user(mut self) -> Self {
        self.name.push_str(&whoami::username());

        self
    }

    /// Appends a derived string from the current working directory.
    pub fn current_dir(mut self) -> Self {
        if let Ok(path) = std::env::current_dir() {
            self.path(path)
        } else {
            self.name.push('_');
            self
        }
    }

    /// Appends a derived string from the given path.
    pub fn path<P: AsRef<Path>>(mut self, path: P) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;

            let hash = mx3::v3::hash(path.as_ref().as_os_str().as_bytes(), 1);

            self.name.push_str(&format!("{:016x}", hash));
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;

            let mut bytes = Vec::with_capacity(path.as_os_str().len() * 2);

            for unit in path.as_os_str().encode_wide() {
                bytes.push((unit >> 8) as u8);
                bytes.push(unit as u8);
            }

            let hash = mx3::v3::hash(&bytes, 1);

            self.name.push_str(&format!("{:016x}", hash));
        }

        self
    }

    /// Appends a derived string from the given name.
    pub fn name<S: AsRef<str>>(mut self, name: S) -> Self {
        let hash = mx3::v3::hash(name.as_ref().as_bytes(), 1);

        self.name.push_str(&format!("{:016x}", hash));

        self
    }

    /// Returns the built string.
    pub fn build(self) -> String {
        self.name
    }
}

impl Default for NameBuilder {
    fn default() -> Self {
        Self::new()
    }
}
