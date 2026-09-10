use crate::remote::RemoteCameraBackend;
use deepsky_camera::{backend::CameraBackend, CameraError, ErrorCode};
use deepsky_testkit::simulator::{SimulatorBackend, TimingMode};
use deepsky_transport::{adb::AdbTransport, tcp::TcpTransport};
#[derive(Debug, Clone)]
pub enum Source { Simulator { realtime: bool }, Tcp(String), Adb(Option<String>) }
impl Source {
    pub fn connect(&self) -> Result<Box<dyn CameraBackend + Send>, CameraError> {
        Ok(match self {
            Self::Simulator { realtime }=>Box::new(SimulatorBackend::new(if *realtime {TimingMode::Realtime}else{TimingMode::Accelerated})),
            Self::Tcp(address)=>{
                let socket:std::net::SocketAddr=address.parse().map_err(|_|CameraError::new(ErrorCode::InvalidRequest,"TCP requires a loopback IP:port"))?;
                if !socket.ip().is_loopback() {return Err(CameraError::new(ErrorCode::Unsupported,"unauthenticated development TCP is restricted to loopback; use an authenticated tunnel"));}
                Box::new(RemoteCameraBackend::connect(Box::new(TcpTransport::new(address)))?)
            }
            Self::Adb(serial)=>Box::new(RemoteCameraBackend::connect(Box::new(AdbTransport::new(serial.clone())))?),
        })
    }
    pub fn accelerated(&self)->bool {matches!(self,Self::Simulator {realtime:false})}
}
