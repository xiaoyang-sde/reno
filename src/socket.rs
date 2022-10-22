use std::fs::remove_file;
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

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
    pub fn bind(path: PathBuf) -> Result<Self, RuntimeError> {
        let listener = UnixListener::bind(&path).map_err(|err| RuntimeError {
            message: format!(
                "failed to bind the UnixListener to {}: {}",
                path.display(),
                err
            ),
        })?;

        Ok(SocketServer {
            path,
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

    pub fn read(&mut self) -> Result<SocketMessage, RuntimeError> {
        let mut buffer = String::new();
        match &mut self.stream {
            Some(stream) => {
                let mut reader = BufReader::new(stream);
                reader.read_line(&mut buffer).map_err(|err| RuntimeError {
                    message: format!("failed to read from the client: {}", err),
                })?;

                let message: SocketMessage =
                    serde_json::from_str(&buffer).map_err(|err| RuntimeError {
                        message: format!("failed to deserialize the client message: {}", err),
                    })?;
                Ok(message)
            }
            None => Err(RuntimeError {
                message: String::from("failed to connect to a client"),
            }),
        }
    }

    pub fn write(&mut self, message: SocketMessage) -> Result<(), RuntimeError> {
        let mut message = serde_json::to_string(&message).map_err(|err| RuntimeError {
            message: format!("failed to serialize the client message: {}", err),
        })?;
        message.push('\n');

        match &mut self.stream {
            Some(stream) => {
                stream
                    .write_all(message.as_bytes())
                    .map_err(|err| RuntimeError {
                        message: format!("failed to write to the client: {}", err),
                    })?;
                stream.flush().map_err(|err| RuntimeError {
                    message: format!("failed to flush the write buffer: {}", err),
                })?;
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
            remove_file(&self.path).unwrap();
        }
    }
}

pub struct SocketClient {
    stream: UnixStream,
}

impl SocketClient {
    pub fn connect(path: PathBuf) -> Result<Self, RuntimeError> {
        let stream = UnixStream::connect(&path).map_err(|err| RuntimeError {
            message: format!("failed to connect to {}: {}", path.display(), err),
        })?;

        Ok(SocketClient { stream })
    }

    pub fn read(&mut self) -> Result<SocketMessage, RuntimeError> {
        let mut buffer = String::new();
        let mut reader = BufReader::new(&self.stream);
        reader.read_line(&mut buffer).map_err(|err| RuntimeError {
            message: format!("failed to read from the client: {}", err),
        })?;

        let message: SocketMessage = serde_json::from_str(&buffer).map_err(|err| RuntimeError {
            message: format!("failed to parse the client message: {}", err),
        })?;
        Ok(message)
    }

    pub fn write(&mut self, message: SocketMessage) -> Result<(), RuntimeError> {
        let mut message = serde_json::to_string(&message).map_err(|err| RuntimeError {
            message: format!("failed to serialize the client message: {}", err),
        })?;
        message.push('\n');

        self.stream
            .write_all(message.as_bytes())
            .map_err(|err| RuntimeError {
                message: format!("failed to write to the client: {}", err),
            })?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), RuntimeError> {
        self.stream
            .shutdown(Shutdown::Both)
            .map_err(|err| RuntimeError {
                message: format!("failed to shutdown the stream: {}", err),
            })?;
        Ok(())
    }
}
