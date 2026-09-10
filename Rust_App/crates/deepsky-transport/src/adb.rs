//! Development transport: dedicated ADB forward, using dynamically allocated host port.
use crate::{Transport, TransportError, tcp::TcpTransport};
use std::process::Command;
pub struct AdbTransport { pub serial: Option<String>, pub remote_port: u16, local_port: Option<u16>, tcp: Option<TcpTransport> }
impl AdbTransport {
    pub fn new(serial: Option<String>) -> Self { Self { serial, remote_port: 7878, local_port: None, tcp: None } }
    fn command(&self) -> Command { let mut cmd=Command::new("adb"); if let Some(serial)=&self.serial { cmd.arg("-s").arg(serial); } cmd }
}
impl Transport for AdbTransport {
    fn connect(&mut self) -> Result<(), TransportError> {
        if self.tcp.is_some() { return Ok(()); }
        let output=self.command().args(["forward","tcp:0",&format!("tcp:{}",self.remote_port)]).output()?;
        if !output.status.success() { return Err(TransportError::Io); }
        let port=String::from_utf8_lossy(&output.stdout).trim().parse::<u16>().map_err(|_|TransportError::Protocol)?;
        self.local_port=Some(port);
        let mut tcp=TcpTransport::new(format!("127.0.0.1:{port}"));
        if let Err(e)=tcp.connect() { let _=self.disconnect(); return Err(e); }
        self.tcp=Some(tcp); Ok(())
    }
    fn send(&mut self,data:&[u8])->Result<(),TransportError>{self.tcp.as_mut().ok_or(TransportError::NotConnected)?.send(data)}
    fn receive(&mut self)->Result<Vec<u8>,TransportError>{self.tcp.as_mut().ok_or(TransportError::NotConnected)?.receive()}
    fn disconnect(&mut self)->Result<(),TransportError>{
        self.tcp.take();
        if let Some(port)=self.local_port.take() {
            let output=self.command().args(["forward","--remove",&format!("tcp:{port}")]).output()?;
            if !output.status.success() { return Err(TransportError::Io); }
        } Ok(())
    }
}
impl Drop for AdbTransport { fn drop(&mut self) { let _=self.disconnect(); } }
