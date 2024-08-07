use crate::{models::AdbCommand, ADBServer, Result, RustADBError};
use std::net::SocketAddrV4;

impl ADBServer {
    /// Connect device over tcp with address and port
    pub fn connect_device(&mut self, address: SocketAddrV4) -> Result<()> {
        let response = self
            .connect()?
            .proxy_connection(AdbCommand::Connect(address), true)?;

        match String::from_utf8(response).unwrap() {
            s if s.starts_with("connected to") => Ok(()),
            s => Err(RustADBError::ADBRequestFailed(s)),
        }
    }
}
