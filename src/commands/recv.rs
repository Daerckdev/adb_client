use crate::{
    models::{AdbCommand, SyncCommand},
    AdbTcpConnection, Result, RustADBError,
};
use byteorder::{ByteOrder, LittleEndian};
use std::io::{Read, Write};

impl AdbTcpConnection {
    /// Receives [path] to [stream] from the device.
    pub fn recv<S: ToString, A: AsRef<str>>(
        &mut self,
        serial: Option<S>,
        path: A,
        stream: &mut dyn Write,
    ) -> Result<()> {
        self.new_connection()?;

        match serial {
            None => self.send_adb_request(AdbCommand::TransportAny)?,
            Some(serial) => {
                self.send_adb_request(AdbCommand::TransportSerial(serial.to_string()))?
            }
        }

        // Set device in SYNC mode
        self.send_adb_request(AdbCommand::Sync)?;

        // Send a recv command
        self.send_sync_request(SyncCommand::Recv)?;

        self.handle_recv_command(path, stream)
    }

    fn handle_recv_command<S: AsRef<str>>(
        &mut self,
        from: S,
        output: &mut dyn Write,
    ) -> Result<()> {
        // First send 8 byte common header
        let mut len_buf = [0_u8; 4];
        LittleEndian::write_u32(&mut len_buf, from.as_ref().len() as u32);
        self.tcp_stream.write_all(&len_buf)?;
        self.tcp_stream.write_all(from.as_ref().as_bytes())?;

        // Then we receive the byte data in chunks of up to 64k
        // Chunk looks like 'DATA' <length> <data>
        let mut buffer = [0_u8; 64 * 1024]; // Should this be Boxed?
        let mut data_header = [0_u8; 4]; // DATA
        loop {
            self.tcp_stream.read_exact(&mut data_header)?;
            // Check if data_header is DATA or DONE or FAIL
            match &data_header {
                b"DATA" => {
                    // Handle received data
                    let length: usize = self.get_body_length()?.try_into().unwrap();
                    self.tcp_stream.read_exact(&mut buffer[..length])?;
                    output.write_all(&buffer)?;
                }
                b"DONE" => break, // We're done here
                b"FAIL" => {
                    // Handle fail
                    let length: usize = self.get_body_length()?.try_into().unwrap();
                    self.tcp_stream.read_exact(&mut buffer[..length])?;
                    Err(RustADBError::ADBRequestFailed(String::from_utf8(
                        buffer[..length].to_vec(),
                    )?))?;
                }
                _ => panic!("Unknown response from device {:#?}", data_header),
            }
        }

        // Connection should've left SYNC by now
        Ok(())
    }
}
