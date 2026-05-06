//! Atomic install of the native-messaging host binary into the user's
//! per-account location.
//!
//! The destination binary may already be running as a child of the user's
//! browser. On Linux that means an in-place overwrite (e.g. `std::fs::copy`'s
//! `O_TRUNC`) hits `ETXTBSY` ("text file busy"). We sidestep the issue by
//! writing a sibling temp file in the same directory and `rename(2)`-ing it
//! over the destination — the running process keeps its old inode alive
//! while subsequent `execve` calls (the next time the browser respawns the
//! host) pick up the new file.

use std::fs;
use std::io;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const COMPARE_BUFFER_SIZE: usize = 16 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    Replaced,
    Unchanged,
}

/// Copy `src` over `dest` atomically, preserving the source's intent of an
/// executable file (mode `0o755` on Unix). Returns [`InstallOutcome::Unchanged`]
/// when the two files are byte-identical, leaving the destination's inode
/// untouched.
pub fn install_messenger_binary(src: &Path, dest: &Path) -> io::Result<InstallOutcome> {
    let dest_dir = dest.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination path has no parent directory",
        )
    })?;
    fs::create_dir_all(dest_dir)?;

    if files_match(src, dest)? {
        return Ok(InstallOutcome::Unchanged);
    }

    let mut temp = tempfile::NamedTempFile::new_in(dest_dir)?;
    {
        let mut src_file = fs::File::open(src)?;
        io::copy(&mut src_file, temp.as_file_mut())?;
    }

    #[cfg(unix)]
    temp.as_file()
        .set_permissions(<fs::Permissions as PermissionsExt>::from_mode(0o755))?;

    temp.as_file().sync_all()?;
    temp.persist(dest).map_err(|e| e.error)?;
    Ok(InstallOutcome::Replaced)
}

fn files_match(a: &Path, b: &Path) -> io::Result<bool> {
    let (Ok(meta_a), Ok(meta_b)) = (fs::metadata(a), fs::metadata(b)) else {
        return Ok(false);
    };
    if meta_a.len() != meta_b.len() {
        return Ok(false);
    }

    let mut file_a = fs::File::open(a)?;
    let mut file_b = fs::File::open(b)?;
    let mut buf_a = vec![0u8; COMPARE_BUFFER_SIZE];
    let mut buf_b = vec![0u8; COMPARE_BUFFER_SIZE];
    loop {
        let n_a = read_chunk(&mut file_a, &mut buf_a)?;
        let n_b = read_chunk(&mut file_b, &mut buf_b)?;
        if n_a != n_b || buf_a[..n_a] != buf_b[..n_b] {
            return Ok(false);
        }
        if n_a == 0 {
            return Ok(true);
        }
    }
}

fn read_chunk(reader: &mut impl io::Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        match reader.read(&mut buf[total..])? {
            0 => break,
            n => total += n,
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn installs_when_destination_missing() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("subdir/dest");
        write(&src, b"new binary contents");

        let outcome = install_messenger_binary(&src, &dest).unwrap();
        assert_eq!(outcome, InstallOutcome::Replaced);
        assert_eq!(fs::read(&dest).unwrap(), b"new binary contents");

        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&dest).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    #[test]
    fn replaces_when_contents_differ() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        write(&src, b"new bytes");
        write(&dest, b"old bytes");

        let outcome = install_messenger_binary(&src, &dest).unwrap();
        assert_eq!(outcome, InstallOutcome::Replaced);
        assert_eq!(fs::read(&dest).unwrap(), b"new bytes");
    }

    #[test]
    fn replaces_when_contents_differ_at_same_length() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        write(&src, b"AAAAAA");
        write(&dest, b"BBBBBB");

        let outcome = install_messenger_binary(&src, &dest).unwrap();
        assert_eq!(outcome, InstallOutcome::Replaced);
        assert_eq!(fs::read(&dest).unwrap(), b"AAAAAA");
    }

    #[test]
    fn skips_when_contents_match() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        write(&src, b"identical bytes");
        write(&dest, b"identical bytes");

        #[cfg(unix)]
        let original_inode = {
            use std::os::unix::fs::MetadataExt;
            fs::metadata(&dest).unwrap().ino()
        };

        let outcome = install_messenger_binary(&src, &dest).unwrap();
        assert_eq!(outcome, InstallOutcome::Unchanged);

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(fs::metadata(&dest).unwrap().ino(), original_inode);
        }
    }

    #[cfg(unix)]
    #[test]
    fn replaces_via_rename_so_open_handles_keep_old_inode() {
        use std::io::{Read, Seek};
        use std::os::unix::fs::MetadataExt;

        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        write(&src, b"new bytes");
        write(&dest, b"old bytes");

        let original_inode = fs::metadata(&dest).unwrap().ino();
        let mut handle = fs::File::open(&dest).unwrap();

        let outcome = install_messenger_binary(&src, &dest).unwrap();
        assert_eq!(outcome, InstallOutcome::Replaced);

        let new_inode = fs::metadata(&dest).unwrap().ino();
        assert_ne!(
            original_inode, new_inode,
            "atomic replace should produce a new inode at the destination path"
        );

        let mut observed = String::new();
        handle.rewind().unwrap();
        handle.read_to_string(&mut observed).unwrap();
        assert_eq!(
            observed, "old bytes",
            "the still-open handle must continue to see the original inode's bytes"
        );
    }

    #[test]
    fn missing_source_surfaces_error() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("does-not-exist");
        let dest = tmp.path().join("dest");
        let err = install_messenger_binary(&src, &dest).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(!dest.exists(), "no destination should be left behind");
    }
}
