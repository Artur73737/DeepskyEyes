//! Bounded disk thread. submit is nonblocking; queue-full returns ownership of the RAW job.
use std::{fs, io, path::{Path, PathBuf}, sync::mpsc::{self, Receiver, SyncSender}, thread::{self, JoinHandle}};
use deepsky_metadata::{frame::FrameMetadata, session_manifest::FrameRecord};
use deepsky_raw::{checksum::sha256_hex, storage::{publish_new, safe_path}};

pub struct DiskWriter {
    pub root: String,
}

impl DiskWriter {
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }
    /// Memory held by worker <= (capacity + 1) * (max_frame_bytes + 1 MiB metadata).
    pub fn start(self, capacity: usize, max_frame_bytes: usize) -> io::Result<DiskWorker> {
        if capacity == 0 || max_frame_bytes == 0 { return Err(io::Error::new(io::ErrorKind::InvalidInput, "positive queue and frame bounds required")); }
        let root = fs::canonicalize(self.root)?;
        if !root.is_dir() { return Err(io::Error::other("disk root is not a directory")); }
        let (tx, rx) = mpsc::sync_channel::<Work>(capacity);
        let handle = thread::Builder::new().name("deepsky-disk".into()).spawn(move || {
            for work in rx {
                let result = write_job(&root, work.job);
                let _ = work.result.send(result);
            }
        })?;
        Ok(DiskWorker { sender: Some(tx), handle: Some(handle), max_frame_bytes })
    }
}

#[derive(Debug)]
pub struct WriteJob {
    pub relative_path: PathBuf,
    /// Encoded DNG supplied by a camera, or unsigned LE16 samples with .raw16 extension.
    pub data: Vec<u8>,
    pub metadata: FrameMetadata,
}
#[derive(Debug)]
pub struct SubmitError { pub kind: io::ErrorKind, pub message: String, pub job: WriteJob }
impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.message) }
}
impl std::error::Error for SubmitError {}
struct Work { job: WriteJob, result: SyncSender<io::Result<FrameRecord>> }
pub struct WriteReceipt { receiver: Receiver<io::Result<FrameRecord>> }
impl WriteReceipt {
    pub fn wait(self) -> io::Result<FrameRecord> {
        self.receiver.recv().map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "disk worker stopped"))?
    }
    pub fn try_result(&self) -> Result<io::Result<FrameRecord>, mpsc::TryRecvError> { self.receiver.try_recv() }
}
pub struct DiskWorker { sender: Option<SyncSender<Work>>, handle: Option<JoinHandle<()>>, max_frame_bytes: usize }
impl DiskWorker {
    pub fn submit(&self, job: WriteJob) -> Result<WriteReceipt, SubmitError> {
        let error = |kind, message: &str, job| SubmitError { kind, message: message.into(), job };
        if job.data.is_empty() || job.data.len() > self.max_frame_bytes {
            return Err(error(io::ErrorKind::InvalidInput, "frame exceeds configured bound or is empty", job));
        }
        if serde_json::to_vec(&job.metadata).map_or(true, |v| v.len() > 1024 * 1024) {
            return Err(error(io::ErrorKind::InvalidInput, "metadata exceeds 1 MiB bound", job));
        }
        let Some(sender) = &self.sender else { return Err(error(io::ErrorKind::BrokenPipe, "disk worker closed", job)); };
        let (tx, receiver) = mpsc::sync_channel(1);
        match sender.try_send(Work { job, result: tx }) {
            Ok(()) => Ok(WriteReceipt { receiver }),
            Err(mpsc::TrySendError::Full(work)) => Err(error(io::ErrorKind::WouldBlock, "disk queue full; retry this job", work.job)),
            Err(mpsc::TrySendError::Disconnected(work)) => Err(error(io::ErrorKind::BrokenPipe, "disk worker stopped", work.job)),
        }
    }
    /// Drain accepted writes and join. Observe every receipt for per-frame errors.
    pub fn shutdown(mut self) -> io::Result<()> { self.finish() }
    fn finish(&mut self) -> io::Result<()> {
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            handle.join().map_err(|_| io::Error::other("disk worker panicked"))?;
        }
        Ok(())
    }
}
impl Drop for DiskWorker { fn drop(&mut self) { let _ = self.finish(); } }

fn write_job(root: &Path, job: WriteJob) -> io::Result<FrameRecord> {
    if job.metadata.frame_id.is_empty() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "frame id required")); }
    let path = safe_path(root, &job.relative_path)?;
    match path.extension().and_then(|s| s.to_str()) {
        Some("raw16") => {
            let m = &job.metadata;
            let size = m.width.zip(m.height).and_then(|(w,h)| (w as u64).checked_mul(h as u64)?.checked_mul(2));
            if size != Some(job.data.len() as u64) || m.width == Some(0) || m.height == Some(0) || m.bits_per_sample != Some(16) || m.byte_order.as_deref() != Some("little") {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "raw16 requires matching dimensions, bits_per_sample=16 and byte_order=little"));
            }
        },
        Some("dng") => {
            // Pass-through only: checking the TIFF header does not certify DNG semantics.
            if job.data.len() < 8 || !(job.data.starts_with(b"II\x2a\0") || job.data.starts_with(b"MM\0\x2a")) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "DNG pass-through requires TIFF header"));
            }
        },
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "use .raw16 or camera-produced .dng")),
    }
    let relative = job.relative_path.to_str().ok_or_else(|| io::Error::other("non UTF-8 path"))?.replace('\\', "/");
    let record = FrameRecord { filename: relative.clone(), size_bytes: job.data.len() as u64,
        timestamp_ns: job.metadata.timestamp_ns, sha256: sha256_hex(&job.data), metadata: job.metadata };
    let sidecar = safe_path(root, Path::new(&format!("{relative}.json")))?;
    // Preflight existing sidecar as well: collisions are never overwritten or removed.
    if sidecar.exists() { return Err(io::Error::new(io::ErrorKind::AlreadyExists, "metadata sidecar exists")); }
    let metadata = serde_json::to_vec_pretty(&serde_json::json!({"schema_version": 1, "frame": record})).map_err(io::Error::other)?;
    fs::create_dir_all(path.parent().unwrap())?;
    publish_new(&path, &job.data)?;
    publish_new(&sidecar, &metadata)?;
    Ok(record)
}
