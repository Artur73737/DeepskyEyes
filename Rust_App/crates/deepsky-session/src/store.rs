//! Append-only, durable manifest revisions. session.json is revision zero;
//! metadata/session-NNNNNNNN.json are successive full snapshots. Use open(), not
//! a direct read of session.json, to obtain the latest committed state.
use std::{fs, io, path::{Path, PathBuf}};
use deepsky_metadata::session_manifest::{FrameRecord, SessionManifest};
use deepsky_raw::{checksum::sha256_reader, storage::{publish_new, safe_path, sync_directory}};
use crate::recovery::{scan, RecoveryReport};

pub struct SessionStore { root: PathBuf, manifest: SessionManifest, revision: u32 }
impl SessionStore {
    /// root must not exist: never reuse an old session implicitly.
    pub fn create(root: impl AsRef<Path>, manifest: SessionManifest) -> io::Result<Self> {
        validate(&manifest)?;
        if !manifest.frames.is_empty() { return Err(invalid("new session must not contain committed frames")); }
        fs::create_dir(root.as_ref())?;
        let root = fs::canonicalize(root)?;
        for dir in ["lights", "darks", "flats", "bias", "test", "metadata"] { fs::create_dir(root.join(dir))?; }
        publish_new(&root.join("metadata/capabilities.json"), &serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1, "capabilities": manifest.capability_snapshot
        })).map_err(io::Error::other)?)?;
        publish_new(&root.join("session.json"), &encode(&manifest)?)?;
        sync_directory(&root)?;
        if let Some(parent) = root.parent() { sync_directory(parent)?; }
        Ok(Self { root, manifest, revision: 0 })
    }
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = fs::canonicalize(root)?;
        let mut manifest = read_manifest(&safe_path(&root, Path::new("session.json"))?)?;
        let mut revisions = Vec::new();
        let directory = safe_path(&root, Path::new("metadata"))?;
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(index) = name.strip_prefix("session-").and_then(|s| s.strip_suffix(".json")) {
                let number: u32 = index.parse().map_err(|_| invalid("invalid manifest revision filename"))?;
                if name != format!("session-{number:08}.json") { return Err(invalid("noncanonical revision filename")); }
                revisions.push(number);
            }
        }
        revisions.sort_unstable();
        let mut revision = 0u32;
        for next in revisions {
            if revision.checked_add(1) != Some(next) { return Err(invalid("manifest revision gap")); }
            let newer = read_manifest(&safe_path(&root, Path::new(&format!("metadata/session-{next:08}.json")))?)?;
            if newer.session_id != manifest.session_id || newer.project != manifest.project ||
                !newer.frames.starts_with(&manifest.frames) {
                return Err(invalid("manifest history changed existing frames or session identity"));
            }
            manifest = newer; revision = next;
        }
        Ok(Self { root, manifest, revision })
    }
    pub fn root(&self) -> &Path { &self.root }
    pub fn manifest(&self) -> &SessionManifest { &self.manifest }
    pub fn scan(&self) -> io::Result<RecoveryReport> { scan(&self.root, &self.manifest) }
    /// Verify receipt against disk before committing a new immutable manifest revision.
    pub fn record_frame(&mut self, record: FrameRecord) -> io::Result<()> {
        let path = safe_path(&self.root, Path::new(&record.filename))?;
        let file = fs::File::open(path)?;
        if file.metadata()?.len() != record.size_bytes || sha256_reader(file)? != record.sha256 {
            return Err(invalid("frame size/hash differs from receipt"));
        }
        let mut next = self.manifest.clone();
        next.frames.push(record);
        next.frames_completed = u32::try_from(next.frames.len()).map_err(io::Error::other)?;
        self.save(next)
    }
    /// Persist configuration/warnings/results while preserving existing frame history.
    pub fn save(&mut self, next: SessionManifest) -> io::Result<()> {
        validate(&next)?;
        if next.session_id != self.manifest.session_id || next.project != self.manifest.project ||
            !next.frames.starts_with(&self.manifest.frames) { return Err(invalid("cannot rewrite session history")); }
        let revision = self.revision.checked_add(1).ok_or_else(|| invalid("revision overflow"))?;
        let path = safe_path(&self.root, Path::new(&format!("metadata/session-{revision:08}.json")))?;
        publish_new(&path, &encode(&next)?)?;
        self.manifest = next; self.revision = revision;
        Ok(())
    }
}
fn invalid(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidData, message) }
fn validate(manifest: &SessionManifest) -> io::Result<()> {
    manifest.validate().map_err(|e| invalid(&e))?;
    for frame in &manifest.frames {
        // Validate portable names separately from filesystem existence.
        if frame.filename.is_empty() || frame.filename.split('/').any(|p| p.is_empty() || p == "." || p == ".." || p.contains([':', '\\'])) {
            return Err(invalid("unsafe frame filename"));
        }
    }
    Ok(())
}
fn encode(manifest: &SessionManifest) -> io::Result<Vec<u8>> { serde_json::to_vec_pretty(manifest).map_err(io::Error::other) }
fn read_manifest(path: &Path) -> io::Result<SessionManifest> {
    // Bound malformed inputs before allocating JSON. Large sessions can raise this explicitly.
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > 64 * 1024 * 1024 { return Err(invalid("manifest exceeds 64 MiB")); }
    let manifest: SessionManifest = serde_json::from_reader(file).map_err(io::Error::other)?;
    validate(&manifest)?;
    Ok(manifest)
}
