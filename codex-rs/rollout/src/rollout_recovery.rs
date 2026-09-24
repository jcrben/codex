use std::fs;
use std::fs::File;
use std::io::Error as IoError;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;

pub(crate) fn recover_rollout_path(path: &Path, mut source: File) -> std::io::Result<(File, bool)> {
    let source_metadata = source.metadata()?;
    match fs::metadata(path) {
        Ok(path_metadata) if same_file(&source_metadata, &path_metadata) => {
            return open_rollout_for_recovery(path).map(|file| (file, false));
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    let parent = path
        .parent()
        .ok_or_else(|| IoError::other(format!("rollout path has no parent: {}", path.display())))?;
    let mut staged = tempfile::Builder::new()
        .prefix(".rollout-recovery-")
        .tempfile_in(parent)?;
    source.seek(SeekFrom::Start(0))?;
    std::io::copy(&mut source, staged.as_file_mut())?;
    staged
        .as_file()
        .set_permissions(source_metadata.permissions())?;
    staged.as_file().sync_all()?;

    match fs::hard_link(staged.path(), path) {
        Ok(()) => {}
        Err(err)
            if err.kind() == std::io::ErrorKind::AlreadyExists
                && rollout_files_match(staged.path(), path)? => {}
        Err(err) => return Err(err),
    }
    sync_parent_directory(parent)?;
    open_rollout_for_recovery(path).map(|file| (file, true))
}

fn open_rollout_for_recovery(path: &Path) -> std::io::Result<File> {
    File::options().read(true).append(true).open(path)
}

fn rollout_files_match(left: &Path, right: &Path) -> std::io::Result<bool> {
    let mut left = File::open(left)?;
    let mut right = File::open(right)?;
    let mut left_buffer = [0; 16 * 1024];
    let mut right_buffer = [0; 16 * 1024];
    loop {
        let left_len = left.read(&mut left_buffer)?;
        let right_len = right.read(&mut right_buffer)?;
        if left_len != right_len || left_buffer[..left_len] != right_buffer[..right_len] {
            return Ok(false);
        }
        if left_len == 0 {
            return Ok(true);
        }
    }
}

pub(crate) fn same_file(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        left.dev() == right.dev() && left.ino() == right.ino()
    }
    #[cfg(not(unix))]
    {
        // Identity checks via file index are nightly-only on Windows. Treat the
        // path as unchanged there; a missing path is still recovered.
        let _ = (left, right);
        true
    }
}

fn sync_parent_directory(parent: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        File::open(parent)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = parent;
        Ok(())
    }
}
