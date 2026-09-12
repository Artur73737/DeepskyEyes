//! Transport-backed camera client and loopback development server.
use deepsky_camera::*;
use deepsky_camera::backend::CameraBackend;
use deepsky_protocol::{codec, framing::Packet, rpc::{self, Reply, Request}};
use deepsky_transport::Transport;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{json, Value};

const BINARY_CAPTURE: u16 = 4;
fn error(message: impl Into<String>) -> CameraError { CameraError::new(ErrorCode::Io, message) }
fn reply_result(reply: Reply) -> CameraResult<Value> {
    match reply {
        Reply::Ok {result} => Ok(result),
        Reply::Error {code,message} => {
            let parsed = serde_json::from_value::<ErrorCode>(Value::String(code.clone())).unwrap_or(ErrorCode::Io);
            Err(CameraError::new(parsed,format!("{code}: {message}")))
        }
    }
}

pub struct RemoteCameraBackend {
    transport: Box<dyn Transport + Send>,
    request: u32,
    outgoing_sequence: u64,
    incoming_sequence: Option<u64>,
    identity: String,
    binary_preview: bool,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}
impl RemoteCameraBackend {
    pub fn connect(mut transport: Box<dyn Transport + Send>) -> CameraResult<Self> {
        transport.connect().map_err(|e| error(e.to_string()))?;
        let mut client=Self { transport, request:0, outgoing_sequence:0, incoming_sequence:None, identity:String::new(), binary_preview:false, rx_bytes:0, tx_bytes:0 };
        let hello: Value=client.call("hello", json!({"protocol_version":1}))?;
        if hello["protocol_version"] != 1 { return Err(error("incompatible protocol")); }
        client.identity=hello["identity"].as_str().ok_or_else(||error("missing device identity"))?.into();
        client.binary_preview=hello["binary_preview"].as_bool().unwrap_or(false);
        Ok(client)
    }
    fn exchange(&mut self, method: &str, params: Value) -> CameraResult<Packet> {
        self.request=self.request.checked_add(1).ok_or_else(||error("request IDs exhausted; reconnect"))?;
        self.outgoing_sequence=self.outgoing_sequence.checked_add(1).ok_or_else(||error("sequence exhausted"))?;
        let payload=codec::encode(&Request { method:method.into(), params }).map_err(|e|error(e.to_string()))?;
        let bytes=Packet::new(rpc::REQUEST,self.request,self.outgoing_sequence,payload).and_then(|p|p.encode()).map_err(|e|error(e.to_string()))?;
        self.transport.send(&bytes).map_err(|e|error(e.to_string()))?; self.tx_bytes+=bytes.len() as u64;
        for _ in 0..1024 {
            let bytes=self.transport.receive().map_err(|e|error(e.to_string()))?; self.rx_bytes+=bytes.len() as u64;
            let packet=Packet::decode(&bytes).map_err(|e|error(e.to_string()))?;
            if self.incoming_sequence.is_some_and(|v| packet.header.sequence_number<=v) { return Err(error("non-monotonic response sequence")); }
            self.incoming_sequence=Some(packet.header.sequence_number);
            if packet.header.message_type==rpc::EVENT { continue; }
            if packet.header.request_id!=self.request { return Err(error("response request ID mismatch")); }
            return Ok(packet);
        }
        Err(error("event limit reached without response"))
    }
    pub fn call<R: DeserializeOwned>(&mut self, method: &str, params: Value) -> CameraResult<R> {
        let packet=self.exchange(method,params)?;
        if packet.header.message_type!=rpc::RESPONSE { return Err(error("unexpected response type")); }
        let reply:Reply=serde_json::from_slice(&packet.payload).map_err(|e|error(format!("RPC {method} JSON: {e}")))?;
        serde_json::from_value(reply_result(reply)?).map_err(|e|error(e.to_string()))
    }
}
impl CameraBackend for RemoteCameraBackend {
    fn id(&self)->&str { &self.identity }
    fn transport_stats(&self)->Option<(u64,u64)> { Some((self.rx_bytes,self.tx_bytes)) }
    fn discover(&mut self)->CameraResult<Vec<CameraCapabilities>> { self.call("discover",Value::Null) }
    fn open(&mut self, selection:&CameraSelection)->CameraResult<()> { self.call("open",json!(selection)) }
    fn configure(&mut self, request:&CaptureRequest, policy:ValidationPolicy)->CameraResult<ConfigurationOutcome> { self.call("configure",json!({"request":request,"policy":policy})) }
    fn capture(&mut self)->CameraResult<CapturedFrame> {
        let packet=self.exchange("capture",Value::Null)?;
        if packet.header.message_type==rpc::RESPONSE {
            let reply:Reply=codec::decode(&packet.payload).map_err(|e|error(e.to_string()))?;
            reply_result(reply)?;
            return Err(error("expected binary capture"));
        }
        if packet.header.message_type!=BINARY_CAPTURE || packet.payload.len()<4 { return Err(error("invalid capture packet")); }
        let n=u32::from_le_bytes(packet.payload[..4].try_into().unwrap()) as usize;
        if n>packet.payload.len()-4 { return Err(error("invalid metadata length")); }
        let metadata=codec::decode(&packet.payload[4..4+n]).map_err(|e|error(e.to_string()))?;
        Ok(CapturedFrame { metadata, payload:packet.payload[4+n..].to_vec() })
    }
    fn preview(&mut self)->CameraResult<PreviewFrame> {
        if !self.binary_preview { return self.call("preview",Value::Null); }
        let packet=self.exchange("preview_binary",Value::Null)?;
        if packet.header.message_type==rpc::RESPONSE {
            let reply:Reply=codec::decode(&packet.payload).map_err(|e|error(e.to_string()))?;
            reply_result(reply)?;
            return Err(error("expected binary preview"));
        }
        if packet.header.message_type!=5 || packet.payload.len()<4 { return Err(error("invalid preview packet")); }
        let n=u32::from_le_bytes(packet.payload[..4].try_into().unwrap()) as usize;
        if n>packet.payload.len()-4 { return Err(error("invalid preview metadata length")); }
        let mut metadata:Value=codec::decode(&packet.payload[4..4+n]).map_err(|e|error(e.to_string()))?;
        metadata["payload"]=json!([]);
        let mut frame:PreviewFrame=serde_json::from_value(metadata).map_err(|e|error(e.to_string()))?;
        frame.payload=packet.payload[4+n..].to_vec();
        Ok(frame)
    }
    fn thermal(&mut self)->CameraResult<ThermalStatus> { self.call("thermal",Value::Null) }
    fn diagnostic(&mut self, method:&str)->CameraResult<Value> {
        if !["status","ping","capability_dump"].contains(&method) {
            return Err(CameraError::new(ErrorCode::Unsupported,"unknown diagnostic"));
        }
        self.call(method,Value::Null)
    }
    fn autofocus_center(&mut self)->CameraResult<u64> { self.call("autofocus_center",Value::Null) }
    fn close(&mut self)->CameraResult<()> { self.call("close",Value::Null) }
}
impl Drop for RemoteCameraBackend { fn drop(&mut self) { let _=self.transport.disconnect(); } }

fn value<T:Serialize>(result:CameraResult<T>)->CameraResult<Value>{serde_json::to_value(result?).map_err(|e|error(e.to_string()))}
fn parse<T:DeserializeOwned>(value:Value)->CameraResult<T>{serde_json::from_value(value).map_err(|e|CameraError::new(ErrorCode::InvalidRequest,e.to_string()))}

/// One camera connection. The caller owns reconnect policy and the backend lifecycle.
pub fn serve_connection(transport:&mut dyn Transport, backend:&mut dyn CameraBackend)->Result<(),String> {
    let mut handshake=false; let mut rx_sequence=0; let mut tx_sequence=0;
    loop {
        let bytes=match transport.receive(){Ok(bytes)=>bytes,Err(deepsky_transport::TransportError::NotConnected)=>return Ok(()),Err(e)=>return Err(e.to_string())};
        let packet=Packet::decode(&bytes).map_err(|e|e.to_string())?;
        if packet.header.message_type!=rpc::REQUEST || packet.header.sequence_number<=rx_sequence {return Err("invalid request sequence/type".into());}
        rx_sequence=packet.header.sequence_number;
        let request:Request=codec::decode(&packet.payload).map_err(|e|e.to_string())?;
        tx_sequence+=1;
        if handshake && request.method=="capture" {
            match backend.capture() {
                Ok(frame)=>{
                    let metadata=codec::encode(&frame.metadata).map_err(|e|e.to_string())?;
                    let mut payload=Vec::with_capacity(4+metadata.len()+frame.payload.len());
                    payload.extend_from_slice(&(metadata.len() as u32).to_le_bytes()); payload.extend_from_slice(&metadata); payload.extend_from_slice(&frame.payload);
                    let response=Packet::new(BINARY_CAPTURE,packet.header.request_id,tx_sequence,payload).and_then(|p|p.encode()).map_err(|e|e.to_string())?;
                    transport.send(&response).map_err(|e|e.to_string())?; continue;
                }
                Err(e)=>{send_reply(transport,packet.header.request_id,tx_sequence,Err(e))?; continue;}
            }
        }
        let result=if request.method=="hello" {
            if request.params["protocol_version"]!=1 {Err(error("incompatible protocol"))} else {handshake=true;Ok(json!({"protocol_version":1,"identity":backend.id()}))}
        } else if !handshake {Err(error("HELLO required"))} else {
            match request.method.as_str() {
                "discover"=>value(backend.discover()),
                "open"=>parse(request.params).and_then(|selection|value(backend.open(&selection))),
                "configure"=>{
                    let r=parse(request.params["request"].clone()); let p=parse(request.params["policy"].clone());
                    r.and_then(|r|p.and_then(|p|value(backend.configure(&r,p))))
                }
                "preview"=>value(backend.preview()), "thermal"=>value(backend.thermal()), "close"=>value(backend.close()),
                "autofocus_center"=>value(backend.autofocus_center()),
                "ping"=>Ok(json!({"echo":request.params,"server_time_ns":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos().min(u64::MAX as u128) as u64})),
                _=>Err(CameraError::new(ErrorCode::Unsupported,"unknown method")),
            }
        };
        send_reply(transport,packet.header.request_id,tx_sequence,result)?;
    }
}
fn send_reply(transport:&mut dyn Transport,id:u32,seq:u64,result:CameraResult<Value>)->Result<(),String>{
    let reply=match result {Ok(result)=>Reply::Ok {result},Err(e)=>Reply::Error {code:format!("{:?}",e.code),message:e.message}};
    let payload=codec::encode(&reply).map_err(|e|e.to_string())?;
    let bytes=Packet::new(rpc::RESPONSE,id,seq,payload).and_then(|p|p.encode()).map_err(|e|e.to_string())?;
    transport.send(&bytes).map_err(|e|e.to_string())
}
