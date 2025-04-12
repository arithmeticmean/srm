use super::cli::SRMArgs;
use super::error::RMSCliError;
use super::error::RMSError;
use crate::error::ExitCode;
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

static VERSION: &str = env!("CARGO_PKG_VERSION");

static HELP_SHORT: &str = "\
                          Try \"srm --help\" for more information
                          ";

static HELP_LONG: &str = "\
  Usage: srm [OPTION]... [FILE]...
Move FILE(s) to Trash.

  -f, --force           ignore nonexistent files and arguments, never prompt
  -r, -R, --recursive   remove directories and their contents recursively
  -d, --dir             remove empty directories
  -v, --verbose         explain what is being done
      --help        display this help and exit
      --version     output version information and exit
";

pub fn srm_run() {
    let mut exitcode = ExitCode::Success;

    match SRMArgs::parse_cmd_from_env() {
        Ok(args) => handle_args(&args, &mut exitcode),
        Err(err) => match err {
            RMSCliError::Help => println!("{HELP_LONG}"),
            RMSCliError::Version => println!("{VERSION}"),
            _ => {
                eprintln!("{err}\n{HELP_SHORT}");
                exitcode.update(ExitCode::UsageError);
            }
        },
    }

    std::process::exit(exitcode.code());
}

const DESTPATH: &str = "some/dir";

fn handle_args(args: &SRMArgs, exitcode: &mut ExitCode) {
    println!("{:?}", args);
    for srcpath in args.paths.iter() {
        match rms_recursive(
            srcpath,
            Path::new(DESTPATH),
            args.isverbose,
            args.isdirectory,
            args.isrecursive,
        ) {
            Ok(_) => (),
            Err(e) => {
                println!("{}", e);
                exitcode.update(ExitCode::RuntimeError);
            }
        }
    }
}

fn rms_empty_dir(srcpath: &Path, destpath: &Path, verbose: bool) -> Result<(), RMSError> {
    if !srcpath.is_dir() {
        return Err(RMSError::SrcError(
            srcpath.to_path_buf(),
            std::io::Error::new(ErrorKind::NotADirectory, "not a directory"),
        ));
    }

    fs::create_dir_all(destpath).map_err(|e| RMSError::DestError(destpath.to_path_buf(), e))?;

    let src_path = srcpath
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))
        .map_err(|e| RMSError::SrcError(srcpath.to_path_buf(), e))?;

    let tmp_dst_path = destpath.join(src_path);
    let dst_path = next_available_filename(&tmp_dst_path)
        .map_err(|e| RMSError::DestError(tmp_dst_path.to_path_buf(), e))?;

    match fs::create_dir(&dst_path) {
        Ok(_) => {
            if let Err(e) = fs::remove_dir(srcpath) {
                fs::remove_dir(&dst_path)
                    .map_err(|e| RMSError::DestError(dst_path.to_path_buf(), e))?;
                return Err(RMSError::SrcError(srcpath.to_path_buf(), e));
            }
        }
        Err(e) => {
            return Err(RMSError::DestError(dst_path, e));
        }
    }

    if verbose {
        println!("removed '{}'", srcpath.display());
    }

    Ok(())
}

fn rms_file(srcpath: &Path, destpath: &Path, verbose: bool) -> Result<(), RMSError> {
    if srcpath.is_dir() {
        return Err(RMSError::SrcError(
            srcpath.to_path_buf(),
            std::io::Error::new(std::io::ErrorKind::IsADirectory, "is a directory"),
        ));
    }

    fs::create_dir_all(destpath).map_err(|e| RMSError::SrcError(srcpath.to_path_buf(), e))?;

    let src_file_name = srcpath
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))
        .map_err(|e| RMSError::SrcError(srcpath.to_path_buf(), e))?;

    let tmp_dst_path = destpath.join(src_file_name);
    let dst_path = next_available_filename(&tmp_dst_path)
        .map_err(|e| RMSError::DestError(tmp_dst_path.to_path_buf(), e))?;

    fs::rename(srcpath, &dst_path).map_err(|e| RMSError::SrcError(srcpath.to_path_buf(), e))?;

    if verbose {
        println!("removed '{}'", srcpath.display());
    }
    Ok(())
}
fn rms_recursive(
    srcpath: &Path,
    destpath: &Path,
    verbose: bool,
    directory: bool,
    recursive: bool,
) -> Result<(), RMSError> {
    if srcpath.is_dir() {
        if recursive {
            let src_name = srcpath
                .file_name()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path")
                })
                .map_err(|e| RMSError::SrcError(srcpath.to_path_buf(), e))?;

            let dst_path = destpath.join(src_name);

            fs::create_dir_all(&dst_path).map_err(|e| RMSError::DestError(dst_path.clone(), e))?;

            for entry in srcpath
                .read_dir()
                .map_err(|e| RMSError::SrcError(srcpath.to_path_buf(), e))?
            {
                let entry = entry.map_err(|e| RMSError::SrcError(PathBuf::new(), e))?;
                rms_recursive(&entry.path(), &dst_path, verbose, directory, recursive)
                    .unwrap_or_else(|e| println!("{}", e));
            }

            fs::remove_dir(srcpath).map_err(|e| RMSError::SrcError(srcpath.to_path_buf(), e))?;
            return Ok(());
        }

        if directory {
            rms_empty_dir(srcpath, destpath, verbose).unwrap_or_else(|e| println!("{}", e));
            return Ok(());
        }
    }

    rms_file(srcpath, destpath, verbose)?;

    Ok(())
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
    use tempfile::TempDir;
    use tempfile::tempdir;

    #[test]
    fn test_rms_empty_dir_success() {
        let tmp_root = tempdir().unwrap();
        let src_dir = tmp_root.path().join("empty_dir");
        let dst_dir = tmp_root.path().join("dest");

        fs::create_dir(&src_dir).unwrap();
        fs::create_dir(&dst_dir).unwrap();

        let result = rms_empty_dir(&src_dir, &dst_dir, false);
        assert!(result.is_ok());

        let moved_dir = dst_dir.join("empty_dir");
        assert!(moved_dir.exists());
        assert!(moved_dir.is_dir());
        assert!(!src_dir.exists());
    }

    #[test]
    fn test_rms_empty_dir_fails_if_source_not_dir() {
        let tmp_root = tempdir().unwrap();
        let fake_dir = tmp_root.path().join("not_a_dir");
        let dst_dir = tmp_root.path().join("dest");

        fs::write(&fake_dir, "I'm not a dir").unwrap();
        fs::create_dir(&dst_dir).unwrap();

        let result = rms_empty_dir(&fake_dir, &dst_dir, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_rms_empty_dir_fails_if_source_not_empty() {
        let tmp_root = tempdir().unwrap();
        let src_dir = tmp_root.path().join("non_empty");
        let dst_dir = tmp_root.path().join("dest");

        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("file.txt"), "data").unwrap();
        fs::create_dir(&dst_dir).unwrap();

        let result = rms_empty_dir(&src_dir, &dst_dir, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_rms_empty_dir_creates_destination_if_missing() {
        let tmp_root = tempdir().unwrap();
        let src_dir = tmp_root.path().join("to_move");
        let dst_dir = tmp_root.path().join("new_dest");

        fs::create_dir(&src_dir).unwrap();

        let result = rms_empty_dir(&src_dir, &dst_dir, false);
        assert!(result.is_ok());

        let moved_dir = dst_dir.join("to_move");
        assert!(moved_dir.exists());
        assert!(dst_dir.exists());
    }

    #[test]
    fn test_rms_empty_dir_fails_if_source_has_no_name() {
        // Simulate invalid path like root ("/") — has no basename
        let result = rms_empty_dir(Path::new("/"), Path::new("/tmp"), false);
        assert!(result.is_err());
    }

    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use tempfile::NamedTempFile;

    fn test_path_exists(path: &Path) -> bool {
        path.exists()
    }

    #[test]
    fn test_remove_file_with_recursive_flag() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        let dest = tempdir().unwrap();

        let result = rms_recursive(&path, dest.path(), false, false, true);
        assert!(result.is_ok());
        assert!(!test_path_exists(&path));
    }

    #[test]
    fn test_remove_empty_directory_with_directory_flag() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let dest = tempdir().unwrap();
        let result = rms_recursive(&path, dest.path(), false, true, false);
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

        let result = rms_recursive(&path, dest.path(), false, false, true);
        assert!(result.is_ok());
        assert!(!test_path_exists(&path));
    }

    #[test]
    fn test_remove_directory_without_recursive_or_directory_should_fail() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let dest = tempdir().unwrap();

        let result = rms_recursive(&path, dest.path(), false, false, false);
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

        let result = rms_recursive(&path, dest.path(), false, false, true);
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

        let result = rms_recursive(&path, dest.path(), false, false, true);
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
        let result = rms_recursive(&child_file, dest.path(), false, false, true);
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
        let result = rms_recursive(&nested_file, dest.path(), false, false, true);
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
        let result = rms_recursive(&file_path, dest.path(), false, false, true);
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
        let result = rms_recursive(&readonly_dir, dest.path(), false, true, false);
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
        let result = rms_recursive(&dir_a, dest.path(), false, false, true);
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
        let result = rms_recursive(&dir_a, dest.path(), false, false, true);
        assert!(result.is_ok());
        assert!(!dir_a.exists());
    }
}
