use deepsky_app::remote::{RemoteCameraBackend, serve_connection};
use deepsky_camera::{backend::CameraBackend, ValidationPolicy};
use deepsky_testkit::simulator::SimulatorBackend;
use deepsky_transport::tcp::TcpTransport;
#[test]
fn real_tcp_discovery_configuration_binary_capture() {
    let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address=listener.local_addr().unwrap();
    let server=std::thread::spawn(move || {
        let (stream,_)=listener.accept().unwrap();
        let mut transport=TcpTransport::from_stream(stream).unwrap();
        serve_connection(&mut transport,&mut SimulatorBackend::default()).unwrap();
    });
    let mut client=RemoteCameraBackend::connect(Box::new(TcpTransport::new(address.to_string()))).unwrap();
    let caps=client.discover().unwrap();
    assert_eq!(caps.len(),1);
    let request=SimulatorBackend::default_request();
    client.open(&request.selection).unwrap();
    let configuration=client.configure(&request,ValidationPolicy::Reject).unwrap();
    assert!(configuration.adjustments.is_empty());
    let frame=client.capture().unwrap();
    assert_eq!(frame.payload.len(),128*96*2);
    assert_eq!(frame.metadata.configuration.requested,request);
    client.close().unwrap();
    drop(client); server.join().unwrap();
}
