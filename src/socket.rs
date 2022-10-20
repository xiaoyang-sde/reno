use std::fs::remove_file;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use crate::error::RuntimeError;

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

    pub fn read(&mut self) -> Result<String, RuntimeError> {
        let mut buffer = String::new();
        match &mut self.stream {
            Some(stream) => {
                stream
                    .read_to_string(&mut buffer)
                    .map_err(|err| RuntimeError {
                        message: format!("failed to read from the client: {}", err),
                    })?;
                Ok(buffer)
            }
            None => Err(RuntimeError {
                message: String::from("failed to connect to a client"),
            }),
        }
    }

    pub fn write(&mut self, message: String) -> Result<(), RuntimeError> {
        match &mut self.stream {
            Some(stream) => {
                stream
                    .write_all(message.as_bytes())
                    .map_err(|err| RuntimeError {
                        message: format!("failed to write to the client: {}", err),
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
    path: PathBuf,
    stream: UnixStream,
}

impl SocketClient {
    pub fn connect(path: PathBuf) -> Result<Self, RuntimeError> {
        let stream = UnixStream::connect(&path).map_err(|err| RuntimeError {
            message: format!("failed to connect to {}: {}", path.display(), err),
        })?;

        Ok(SocketClient { path, stream })
    }

    pub fn read(&mut self) -> Result<String, RuntimeError> {
        let mut buffer = String::new();
        self.stream
            .read_to_string(&mut buffer)
            .map_err(|err| RuntimeError {
                message: format!("failed to read from the client: {}", err),
            })?;
        Ok(buffer)
    }

    pub fn write(&mut self, message: String) -> Result<(), RuntimeError> {
        self.stream
            .write_all(message.as_bytes())
            .map_err(|err| RuntimeError {
                message: format!("failed to write to the client: {}", err),
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::socket::{SocketClient, SocketServer};
    use std::thread;
    use std::{fs::remove_file, path::Path};

    #[test]
    fn test_server_write() {
        let socket_path = Path::new("/tmp/test_server_write.sock");
        if socket_path.try_exists().unwrap() {
            remove_file(socket_path).unwrap();
        }

        let mut server = SocketServer::bind(socket_path.to_path_buf()).unwrap();
        let mut client = SocketClient::connect(socket_path.to_path_buf()).unwrap();

        let server_thread = thread::spawn(move || {
            server.listen().unwrap();
            server.write(String::from("test_server_write")).unwrap();
        });

        let client_thread = thread::spawn(move || {
            assert_eq!(client.read().unwrap(), String::from("test_server_write"));
        });

        server_thread.join().unwrap();
        client_thread.join().unwrap();
    }

    #[test]
    fn test_client_write() {
        let socket_path = Path::new("/tmp/test_client_write.sock");
        if socket_path.try_exists().unwrap() {
            remove_file(socket_path).unwrap();
        }

        let mut server = SocketServer::bind(socket_path.to_path_buf()).unwrap();
        let mut client = SocketClient::connect(socket_path.to_path_buf()).unwrap();

        let server_thread = thread::spawn(move || {
            server.listen().unwrap();
            assert_eq!(server.read().unwrap(), String::from("test_client_write"));
        });

        let client_thread = thread::spawn(move || {
            client.write(String::from("test_client_write")).unwrap();
        });

        server_thread.join().unwrap();
        client_thread.join().unwrap();
    }
}
