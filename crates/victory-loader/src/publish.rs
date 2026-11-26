//! Portable publication of derived files into a shared cache directory.
//!
//! Several copies of the runner can prepare the same title at once. Each
//! writer stages into its own file, so two of them cannot truncate each
//! other's bytes or rename a file the other already published. The final
//! rename is atomic within the directory, and the staging file is removed
//! if publication fails.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_STAGING: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` unless it is already there, using a staging file
/// unique to this process and call.
pub fn write_if_changed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    publish(path, &NEXT_STAGING, |file| file.write_all(bytes))
}

fn staging_name(path: &Path, sequence: u64) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    path.with_file_name(format!(".{name}.{}.{sequence}.tmp", std::process::id()))
}

fn create_staging(path: &Path, counter: &AtomicU64) -> io::Result<(PathBuf, File)> {
    for _ in 0..128 {
        let staging = staging_name(path, counter.fetch_add(1, Ordering::Relaxed));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
        {
            Ok(file) => return Ok((staging, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique image staging file after 128 attempts",
    ))
}

fn publish(
    path: &Path,
    counter: &AtomicU64,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let (staging, mut file) = create_staging(path, counter)?;
    let written = write(&mut file);
    drop(file);
    let published = written.and_then(|()| std::fs::rename(&staging, path));
    if published.is_err() {
        let _ = std::fs::remove_file(&staging);
    }
    published
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two publishers staging the same destination at once both succeed,
    /// and neither leaves a staging file behind.
    #[test]
    fn concurrent_publishers_do_not_share_a_staging_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.bin");
        let bytes = b"a prepared image".to_vec();
        let (first, second) = (path.clone(), path.clone());
        let (first_bytes, second_bytes) = (bytes.clone(), bytes.clone());
        let (one, two) = (
            std::thread::spawn(move || write_if_changed(&first, &first_bytes)),
            std::thread::spawn(move || write_if_changed(&second, &second_bytes)),
        );
        one.join().unwrap().unwrap();
        two.join().unwrap().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path() != path)
            .collect();
        assert!(leftovers.is_empty(), "staging files were removed: {leftovers:?}");
    }

    #[test]
    fn a_failed_publication_removes_its_staging_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.bin");
        std::fs::create_dir(&path).unwrap();
        assert!(write_if_changed(&path, b"bytes").is_err());
        assert!(path.is_dir());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn stale_staging_files_are_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.bin");
        let counter = AtomicU64::new(0);
        for n in 0..2 {
            std::fs::write(staging_name(&path, n), b"stale").unwrap();
        }
        publish(&path, &counter, |file| file.write_all(b"fresh")).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"fresh");
        for n in 0..2 {
            assert_eq!(std::fs::read(staging_name(&path, n)).unwrap(), b"stale");
        }
        assert!(!staging_name(&path, 2).exists());
        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn failed_write_removes_only_its_own_staging_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.bin");
        let counter = AtomicU64::new(0);
        std::fs::write(&path, b"original").unwrap();
        std::fs::write(staging_name(&path, 0), b"stale").unwrap();
        let error = publish(&path, &counter, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("simulated write failure"))
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "simulated write failure");
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        assert_eq!(std::fs::read(staging_name(&path, 0)).unwrap(), b"stale");
        assert!(!staging_name(&path, 1).exists());
    }

    #[test]
    fn staging_collisions_have_a_retry_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.bin");
        let counter = AtomicU64::new(0);
        for n in 0..128 {
            std::fs::write(staging_name(&path, n), b"stale").unwrap();
        }
        let error = publish(&path, &counter, |_| panic!("must not write")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(counter.load(Ordering::Relaxed), 128);
        assert!(!path.exists());
        for n in 0..128 {
            assert_eq!(std::fs::read(staging_name(&path, n)).unwrap(), b"stale");
        }
    }

    #[test]
    fn other_staging_errors_are_not_retried() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing-parent").join("image.bin");
        let counter = AtomicU64::new(0);
        let error = publish(&path, &counter, |_| panic!("must not write")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
