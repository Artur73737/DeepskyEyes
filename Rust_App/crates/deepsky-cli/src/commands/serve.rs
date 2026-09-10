//! CLI serve: expose the SYNTHETIC simulator on TCP loopback for remote-path
//! testing (framed packets, real sockets). Identity stays SYNTHETIC.
use std::net::TcpListener;

use deepsky_app::remote::serve_connection;
use deepsky_testkit::SimulatorBackend;
use deepsky_transport::{tcp::TcpTransport, Transport};

pub fn run(args: &[String]) -> Result<(), String> {
    let port = crate::flag_or(args, "port", 0)?;
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("bind: {e}"))?;
    let addr = listener.local_addr().map_err(|e| format!("local_addr: {e}"))?;
    println!("serving SYNTHETIC simulator on {addr} (Ctrl-C to stop)");
    let mut simulator = SimulatorBackend::default();
    for stream in listener.incoming() {
        let stream = stream.map_err(|e| format!("accept: {e}"))?;
        let mut transport = TcpTransport::from_stream(stream).map_err(|e| e.to_string())?;
        if let Err(e) = serve_connection(&mut transport, &mut simulator) {
            eprintln!("serve: connection error: {e}");
        }
        let _ = transport.disconnect();
    }
    Ok(())
}
