use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use crate::state::Status;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketMessage {
    pub status: Status,
    pub error: Option<String>,
}

impl SocketMessage {
    pub fn new(status: Status, error: Option<String>) -> Self {
        SocketMessage { status, error }
    }
}

pub struct SocketServer {
    path: PathBuf,
    listener: UnixListener,
    stream: Option<UnixStream>,
}

impl SocketServer {
    pub fn bind(path: &Path) -> Result<Self> {
        let listener =
            UnixListener::bind(path).context(format!("failed to bind to {}", path.display()))?;
        Ok(SocketServer {
            path: path.to_path_buf(),
            listener,
            stream: None,
        })
    }

    pub fn listen(&mut self) -> Result<()> {
        match self.listener.accept() {
            Ok((stream, _)) => self.stream = Some(stream),
            Err(_err) => bail!("failed to accept the incoming connection"),
        }
        Ok(())
    }

    pub fn write(&mut self, message: SocketMessage) -> Result<()> {
        let mut message =
            serde_json::to_string(&message).context("failed to serialize the client message")?;
        message.push('\n');

        match &mut self.stream {
            Some(stream) => {
                stream
                    .write_all(message.as_bytes())
                    .context("failed to send the message to the client")?;
                stream.flush().context("failed to flush the write buffer")?;
                Ok(())
            }
            None => bail!("failed to connect to a client"),
        }
    }
}

impl Drop for SocketServer {
    fn drop(&mut self) {
        if self.path.try_exists().unwrap() {
            fs::remove_file(&self.path).unwrap();
        }
    }
}

pub struct SocketClient {
    stream: UnixStream,
}

impl SocketClient {
    pub fn connect(path: &Path) -> Result<Self> {
        let stream = UnixStream::connect(path).context("failed to connect to the server")?;
        Ok(SocketClient { stream })
    }

    pub fn read(&mut self) -> Result<SocketMessage> {
        let mut buffer = String::new();
        let mut reader = BufReader::new(&self.stream);
        reader
            .read_line(&mut buffer)
            .context("failed to read the message from the server")?;

        let message: SocketMessage =
            serde_json::from_str(&buffer).context("failed to parse the client message")?;
        Ok(message)
    }

    pub fn shutdown(&self) -> Result<()> {
        self.stream
            .shutdown(Shutdown::Both)
            .context("failed to shutdown the connection")?;
        Ok(())
    }
}
