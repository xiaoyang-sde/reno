use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RuntimeError;
use crate::state::Status;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketMessage {
    pub status: Status,
    pub error: Option<RuntimeError>,
}

pub struct SocketServer {
    path: PathBuf,
    listener: UnixListener,
    stream: Option<UnixStream>,
}

impl SocketServer {
    pub fn bind(path: &Path) -> Result<Self, RuntimeError> {
        let listener = UnixListener::bind(path)?;
        Ok(SocketServer {
            path: path.to_path_buf(),
            listener,
            stream: None,
        })
    }

    pub fn listen(&mut self) -> Result<(), RuntimeError> {
        match self.listener.accept() {
            Ok((stream, _)) => self.stream = Some(stream),
            Err(err) => {
                return Err(RuntimeError {
                    message: format!("failed to accept the incoming connection: {}", err),
                })
            }
        }
        Ok(())
    }

    pub fn write(&mut self, message: SocketMessage) -> Result<(), RuntimeError> {
        let mut message = serde_json::to_string(&message).map_err(|err| RuntimeError {
            message: format!("failed to serialize the client message: {}", err),
        })?;
        message.push('\n');

        match &mut self.stream {
            Some(stream) => {
                stream.write_all(message.as_bytes())?;
                stream.flush()?;
                Ok(())
            }
            None => Err(RuntimeError {
                message: String::from("failed to connect to a client"),
            }),
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
    pub fn connect(path: &Path) -> Result<Self, RuntimeError> {
        let stream = UnixStream::connect(path)?;
        Ok(SocketClient { stream })
    }

    pub fn read(&mut self) -> Result<SocketMessage, RuntimeError> {
        let mut buffer = String::new();
        let mut reader = BufReader::new(&self.stream);
        reader.read_line(&mut buffer)?;

        let message: SocketMessage = serde_json::from_str(&buffer).map_err(|err| RuntimeError {
            message: format!("failed to parse the client message: {}", err),
        })?;
        Ok(message)
    }

    pub fn shutdown(&self) -> Result<(), RuntimeError> {
        self.stream.shutdown(Shutdown::Both)?;
        Ok(())
    }
}
