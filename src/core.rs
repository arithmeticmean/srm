use crate::error::SRMRuntimeError;
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

fn srm_empty_dir(
    srcpath: &Path,
    destpath: &Path,
    destpath_name: &Path,
    verbose: bool,
) -> Result<(), SRMRuntimeError> {
    fs::create_dir_all(destpath)
        .map_err(|e| SRMRuntimeError::DestError(destpath.to_path_buf(), e))?;

    match fs::create_dir(&destpath_name) {
        Ok(_) => {
            if let Err(e) = fs::remove_dir(srcpath) {
                fs::remove_dir(&destpath_name)
                    .map_err(|e| SRMRuntimeError::DestError(destpath.to_path_buf(), e))?;
                return Err(SRMRuntimeError::SrcError(srcpath.to_path_buf(), e));
            }
        }
        Err(e) => {
            return Err(SRMRuntimeError::DestError(destpath_name.to_path_buf(), e));
        }
    }

    if verbose {
        println!("removed '{}'", srcpath.display());
    }

    Ok(())
}

fn srm_file(
    srcpath: &Path,
    destpath: &Path,
    destpath_name: &Path,
    verbose: bool,
) -> Result<(), SRMRuntimeError> {
    if srcpath.is_dir() {
        return Err(SRMRuntimeError::SrcError(
            srcpath.to_path_buf(),
            std::io::Error::from(ErrorKind::IsADirectory),
        ));
    }

    fs::create_dir_all(destpath)
        .map_err(|e| SRMRuntimeError::SrcError(srcpath.to_path_buf(), e))?;

    fs::rename(srcpath, &destpath_name)
        .map_err(|e| SRMRuntimeError::SrcError(srcpath.to_path_buf(), e))?;

    if verbose {
        println!("removed '{}'", srcpath.display());
    }
    Ok(())
}

fn srm_recursive(
    srcpath: &Path,
    destpath: &Path,
    destpath_name: &Path,
    verbose: bool,
    directory: bool,
    recursive: bool,
) -> Result<(), SRMRuntimeError> {
    if srcpath.is_dir() {
        if recursive {
            let srcpath_name = srcpath
                .file_name()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path")
                })
                .map_err(|e| SRMRuntimeError::SrcError(srcpath.to_path_buf(), e))?;

            let destpath_name_new = destpath_name.join(srcpath_name);

            fs::create_dir_all(&destpath_name)
                .map_err(|e| SRMRuntimeError::DestError(destpath_name.to_path_buf(), e))?;

            for entry in srcpath
                .read_dir()
                .map_err(|e| SRMRuntimeError::SrcError(srcpath.to_path_buf(), e))?
            {
                let entry = entry.map_err(|e| SRMRuntimeError::SrcError(PathBuf::new(), e))?;
                if let Err(e) = srm_recursive(
                    &entry.path(),
                    &destpath,
                    &destpath_name_new,
                    verbose,
                    directory,
                    recursive,
                ) {
                    println!("{}", e);
                }
            }

            fs::remove_dir(srcpath)
                .map_err(|e| SRMRuntimeError::SrcError(srcpath.to_path_buf(), e))?;

            return Ok(());
        }

        if directory {
            return srm_empty_dir(srcpath, destpath, destpath_name, verbose);
        }
    }

    return srm_file(srcpath, destpath, destpath_name, verbose);
}

pub fn srm(
    srcpath: &Path,
    destpath: &Path,
    isverbose: bool,
    isdirectory: bool,
    isrecursive: bool,
) -> Result<(PathBuf, PathBuf), SRMRuntimeError> {
    let src_path = srcpath
        .canonicalize()
        .map_err(|e| SRMRuntimeError::SrcError(srcpath.to_path_buf(), e))?;

    let srcpath_filename = srcpath
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))
        .map_err(|e| SRMRuntimeError::SrcError(srcpath.to_path_buf(), e))?;

    let tmp_destpath_filename = destpath.join(srcpath_filename);

    let destpath_filename = next_available_filename(&tmp_destpath_filename)
        .map_err(|e| SRMRuntimeError::DestError(tmp_destpath_filename.to_path_buf(), e))?;

    srm_recursive(
        &srcpath,
        destpath,
        &destpath_filename,
        isverbose,
        isdirectory,
        isrecursive,
    )
    .map(|_| (src_path, destpath_filename))
}

fn next_available_filename<P: AsRef<Path>>(original: P) -> std::io::Result<PathBuf> {
    let original = original.as_ref();

    if !original.exists() {
        return Ok(original.to_path_buf());
    }

    let parent = original.parent().unwrap_or_else(|| Path::new(""));

    let filename_os = original
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;

    let filename = filename_os.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "filename is not valid UTF-8",
        )
    })?;

    let mut counter = 1;
    let mut buf = String::with_capacity(filename.len() + 10);

    loop {
        buf.clear();
        write!(&mut buf, "{}.{}", filename, counter).unwrap();

        let new_path = parent.join(&buf);
        if !new_path.exists() {
            return Ok(new_path);
        }

        counter += 1;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use tempfile::NamedTempFile;
    use tempfile::tempdir;

    fn test_path_exists(path: &Path) -> bool {
        path.exists()
    }

    #[test]
    fn test_remove_file_with_recursive_flag() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        let dest = tempdir().unwrap();

        let result = srm(&path, dest.path(), false, false, true);
        assert!(result.is_ok());
        assert!(!test_path_exists(&path));
    }

    #[test]
    fn test_remove_empty_directory_with_directory_flag() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let dest = tempdir().unwrap();
        let result = srm(&path, dest.path(), false, true, false);
        assert!(result.is_ok());
        assert!(!test_path_exists(&path));
    }

    #[test]
    fn test_remove_directory_recursively() {
        let dir = tempdir().unwrap();
        let nested_file = dir.path().join("file.txt");
        File::create(&nested_file).unwrap();

        let path = dir.path().to_path_buf();
        let dest = tempdir().unwrap();

        let result = srm(&path, dest.path(), false, false, true);
        assert!(result.is_ok());
        assert!(!test_path_exists(&path));
    }

    #[test]
    fn test_remove_directory_without_recursive_or_directory_should_fail() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let dest = tempdir().unwrap();

        let result = srm(&path, dest.path(), false, false, false);
        assert!(result.is_err());
        assert!(test_path_exists(&path));
    }

    #[test]
    fn test_remove_file_with_permission_denied_should_suceed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("testfile");
        fs::write(&path, b"data").unwrap();

        let mut dir_perms = fs::metadata(dir.path()).unwrap().permissions();
        dir_perms.set_mode(0o700);
        fs::set_permissions(dir.path(), dir_perms).unwrap();

        let dest = tempdir().unwrap();
        let dest_path = dest.path().to_path_buf();

        let result = srm(&path, dest.path(), false, false, true);
        assert!(result.is_ok());
        assert!(dest_path.join("testfile").exists());

        let mut dir_perms = fs::metadata(dir.path()).unwrap().permissions();
        dir_perms.set_mode(0o755);
        let _ = fs::set_permissions(dir.path(), dir_perms);
    }

    #[test]
    fn test_remove_directory_with_permission_denied() {
        let dir = tempdir().unwrap();
        let nested_file = dir.path().join("file.txt");
        File::create(&nested_file).unwrap();

        // Restrict directory permissions
        let mut perms = fs::metadata(dir.path()).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(dir.path(), perms).unwrap();

        let path = dir.path().to_path_buf();
        let dest = tempdir().unwrap();

        let result = srm(&path, dest.path(), false, false, true);
        assert!(result.is_err());

        // Restore permissions
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(&path, perms);
    }

    #[test]
    fn test_no_write_permission_on_parent_directory() {
        let parent_dir = tempdir().unwrap();
        let child_file = parent_dir.path().join("protected.txt");
        File::create(&child_file).unwrap();

        // Remove write permission on parent dir
        let mut perms = fs::metadata(parent_dir.path()).unwrap().permissions();
        perms.set_mode(0o500); // Read and execute, no write
        fs::set_permissions(parent_dir.path(), perms).unwrap();

        let dest = tempdir().unwrap();
        let result = srm(&child_file, dest.path(), false, false, true);
        assert!(result.is_err());

        // Restore permission so tempdir can clean up
        let mut perms = fs::metadata(parent_dir.path()).unwrap().permissions();
        perms.set_mode(0o700);
        let _ = fs::set_permissions(parent_dir.path(), perms);
    }

    #[test]
    fn test_directory_without_execute_permission_should_fail() {
        let dir = tempdir().unwrap();
        let nested_file = dir.path().join("invisible.txt");
        File::create(&nested_file).unwrap();

        let mut perms = fs::metadata(dir.path()).unwrap().permissions();
        perms.set_mode(0o600); // No execute — can't traverse
        fs::set_permissions(dir.path(), perms).unwrap();

        let dest = tempdir().unwrap();
        let result = srm(&nested_file, dest.path(), false, false, true);
        assert!(result.is_err());

        let mut perms = fs::metadata(dir.path()).unwrap().permissions();
        perms.set_mode(0o700);
        let _ = fs::set_permissions(dir.path(), perms);
    }

    #[test]
    fn test_read_only_file_inside_writable_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("readonly.txt");
        let file = File::create(&file_path).unwrap();

        // Set file to read-only
        let mut perms = file.metadata().unwrap().permissions();
        perms.set_mode(0o400);
        fs::set_permissions(&file_path, perms).unwrap();

        let dest = tempdir().unwrap();
        let result = srm(&file_path, dest.path(), false, false, true);
        assert!(result.is_ok());
        assert!(!file_path.exists());
    }

    #[test]
    fn test_read_only_directory_with_writable_parent() {
        let parent_dir = tempdir().unwrap();
        let readonly_dir = parent_dir.path().join("readonly_dir");
        fs::create_dir(&readonly_dir).unwrap();

        let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
        perms.set_mode(0o500);
        fs::set_permissions(&readonly_dir, perms).unwrap();

        let dest = tempdir().unwrap();
        let result = srm(&readonly_dir, dest.path(), false, true, false);
        assert!(result.is_ok());
        assert!(!readonly_dir.exists());
    }

    #[test]
    fn test_nested_mixed_permission_tree_recursive_delete_should_fail() {
        let root = tempdir().unwrap();
        let dir_a = root.path().join("a");
        let dir_b = dir_a.join("b");
        let file = dir_b.join("deep.txt");

        fs::create_dir(&dir_a).unwrap();
        fs::create_dir(&dir_b).unwrap();
        File::create(&file).unwrap();

        fs::set_permissions(&dir_a, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&dir_b, fs::Permissions::from_mode(0o500)).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o400)).unwrap();

        let dest = tempdir().unwrap();
        let result = srm(&dir_a, dest.path(), false, false, true);
        assert!(result.is_err());
        assert!(dir_a.exists());
    }
    #[test]
    fn test_nested_mixed_permission_tree_recursive_delete() {
        let root = tempdir().unwrap();
        let dir_a = root.path().join("a");
        let dir_b = dir_a.join("b");
        let file = dir_b.join("deep.txt");

        fs::create_dir(&dir_a).unwrap();
        fs::create_dir(&dir_b).unwrap();
        File::create(&file).unwrap();

        fs::set_permissions(&dir_a, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&dir_b, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o400)).unwrap();

        let dest = tempdir().unwrap();
        let result = srm(&dir_a, dest.path(), false, false, true);
        assert!(result.is_ok());
        assert!(!dir_a.exists());
    }
}
