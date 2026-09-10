//! Crash recovery: scan/verifica/ricostruzione/resume senza overwrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDecision {
    Resume,
    StartFresh,
    Abort,
}

use std::{collections::HashSet, fs, io, path::{Path, PathBuf}};
use deepsky_metadata::session_manifest::SessionManifest;
use deepsky_raw::{checksum::sha256_reader, storage::{is_link, safe_path}};

#[derive(Debug, Clone, Default)]
pub struct RecoveryReport {
    pub verified: Vec<String>,
    pub missing: Vec<String>,
    pub corrupt: Vec<String>,
    /// Orphan payloads/sidecars and interrupted .pending writes require review.
    pub untracked: Vec<String>,
    pub frames_remaining: u32,
}
impl RecoveryReport {
    /// Integrity gate only. Caller must ALSO revalidate camera and capture configuration.
    pub fn can_resume(&self) -> bool { self.missing.is_empty() && self.corrupt.is_empty() && self.untracked.is_empty() }
}
pub fn scan(root: impl AsRef<Path>, manifest: &SessionManifest) -> io::Result<RecoveryReport> {
    manifest.validate().map_err(io::Error::other)?;
    let root = fs::canonicalize(root)?;
    let mut report = RecoveryReport::default();
    let mut known = HashSet::new();
    for frame in &manifest.frames {
        known.insert(frame.filename.clone());
        let path = safe_path(&root, Path::new(&frame.filename))?;
        match fs::File::open(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => report.missing.push(frame.filename.clone()),
            Err(e) => return Err(e),
            Ok(file) => {
                if file.metadata()?.len() != frame.size_bytes || sha256_reader(file)? != frame.sha256 {
                    report.corrupt.push(frame.filename.clone());
                } else { report.verified.push(frame.filename.clone()); }
            }
        }
        let sidecar = format!("{}.json", frame.filename);
        let sidecar_path = safe_path(&root, Path::new(&sidecar))?;
        if sidecar_path.exists() {
            known.insert(sidecar.clone());
            let file = fs::File::open(sidecar_path)?;
            if file.metadata()?.len() > 2 * 1024 * 1024 { report.corrupt.push(sidecar); continue; }
            let value = serde_json::from_reader::<_, serde_json::Value>(file);
            let valid = value.ok().filter(|v| v["schema_version"] == 1)
                .and_then(|v| serde_json::from_value::<deepsky_metadata::session_manifest::FrameRecord>(v["frame"].clone()).ok())
                .is_some_and(|record| &record == frame);
            if !valid { report.corrupt.push(sidecar); }
        }
    }
    fn walk(root: &Path, dir: &Path, files: &mut Vec<String>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            let relative: PathBuf = entry.path().strip_prefix(root).map_err(io::Error::other)?.into();
            if is_link(&metadata) { files.push(relative.to_string_lossy().replace('\\', "/")); }
            else if metadata.is_dir() { walk(root, &entry.path(), files)?; }
            else { files.push(relative.to_string_lossy().replace('\\', "/")); }
        }
        Ok(())
    }
    let mut files = vec![];
    walk(&root, &root, &mut files)?;
    for file in files {
        let revision = file.strip_prefix("metadata/session-").and_then(|s| s.strip_suffix(".json"))
            .is_some_and(|s| s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit()));
        if !known.contains(&file) && file != "session.json" && file != "metadata/capabilities.json" && !revision {
            report.untracked.push(file);
        }
    }
    report.verified.sort(); report.missing.sort(); report.corrupt.sort(); report.untracked.sort();
    report.frames_remaining = manifest.frames_requested.saturating_sub(report.verified.len() as u32);
    Ok(report)
}
