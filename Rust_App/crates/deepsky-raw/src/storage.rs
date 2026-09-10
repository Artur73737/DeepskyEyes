//! No-clobber file publication. Session directories must not have concurrent hostile writers.
use std::{fs::{self, OpenOptions}, io::{self, Write}, path::{Component, Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};
static NEXT: AtomicU64 = AtomicU64::new(0);

/// Reject traversal, Windows ADS/prefixes and symlink/reparse targets.
pub fn safe_path(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    if relative.as_os_str().is_empty() || !relative.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "path must be a nonempty relative path"));
    }
    let mut path = root.to_path_buf();
    for component in relative.components() {
        let value = component.as_os_str().to_str().ok_or_else(|| io::Error::other("path is not UTF-8"))?;
        if value.contains([':', '\\']) || value.ends_with(['.', ' ']) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsafe filename"));
        }
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(m) if is_link(&m) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "symlink/reparse path rejected")),
            Ok(_) => {},
            Err(e) if e.kind() == io::ErrorKind::NotFound => {},
            Err(e) => return Err(e),
        }
    }
    Ok(path)
}

pub fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))] { metadata.file_type().is_symlink() }
}

pub fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)] { fs::File::open(path)?.sync_all() }
    // Windows Rust std cannot flush directory handles. All file contents are FlushFileBuffers'd;
    // recovery detects missing directory entries after a machine/power failure.
    #[cfg(not(unix))] { let _ = path; Ok(()) }
}

/// Publish complete flushed bytes using an atomic hard-link that NEVER replaces a destination.
/// Unsupported filesystems return an error rather than downgrade to an overwriting rename.
pub fn publish_new(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::other("missing parent"))?;
    let temp = parent.join(format!(".pending-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)?;
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::hard_link(&temp, path)?;
        sync_directory(parent)
    })();
    let removed = fs::remove_file(&temp);
    result?;
    removed?;
    sync_directory(parent)
}
