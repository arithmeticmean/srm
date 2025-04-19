use crate::error::SRMRuntimeError;
use filetime::FileTime;
use filetime::set_file_times;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn move_to_trash_empty_dir(
    srcpath: &Path,
    destpath: &Path,
    verbose: bool,
) -> Result<(), SRMRuntimeError> {
    if !srcpath.is_dir() {
        return Err(io::Error::from(io::ErrorKind::NotADirectory))
            .map_err(|e| SRMRuntimeError::new_src_error(srcpath.to_path_buf(), e));
    }

    fs::create_dir(&destpath)
        .map_err(|e| SRMRuntimeError::new_trash_error(srcpath.to_path_buf(), e))?;

    if let Ok(meta) = fs::metadata(srcpath) {
        let _ = fs::set_permissions(&destpath, meta.permissions());
        let mtime = FileTime::from_last_modification_time(&meta);
        let atime = FileTime::from_last_access_time(&meta);
        let _ = set_file_times(&destpath, atime, mtime);
    }
    match fs::remove_dir(srcpath) {
        Ok(()) => (),
        Err(e) => {
            let _ = fs::remove_dir(destpath)
                .map_err(|e| SRMRuntimeError::new_trash_error(srcpath.to_path_buf(), e))?;
            return Err(SRMRuntimeError::new_src_error(srcpath.to_path_buf(), e));
        }
    }

    if verbose {
        println!("removed '{}'", srcpath.display());
    }

    Ok(())
}

fn move_to_trash_file(
    srcpath: &Path,
    destpath: &Path,
    verbose: bool,
) -> Result<(), SRMRuntimeError> {
    if srcpath.is_dir() {
        return Err(SRMRuntimeError::new_src_error(
            srcpath.to_path_buf(),
            std::io::Error::from(io::ErrorKind::IsADirectory),
        ));
    }

    if !srcpath.exists() {
        return Err(SRMRuntimeError::new_src_error(
            srcpath.to_path_buf(),
            io::Error::from(io::ErrorKind::NotFound),
        ));
    }

    if let Some(parent) = destpath.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| SRMRuntimeError::new_trash_error(srcpath.to_path_buf(), e))?;
    }

    fs::copy(srcpath, &destpath)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        .map_err(|e| SRMRuntimeError::new_trash_error(srcpath.to_path_buf(), e))?;

    if let Ok(meta) = fs::metadata(srcpath) {
        let _ = fs::set_permissions(&destpath, meta.permissions());
        let mtime = FileTime::from_last_modification_time(&meta);
        let atime = FileTime::from_last_access_time(&meta);
        let _ = set_file_times(&destpath, atime, mtime);
    }

    match fs::remove_file(srcpath) {
        Ok(()) => (),
        Err(e) => {
            let _ = fs::remove_file(&destpath)
                .map_err(|e| SRMRuntimeError::new_trash_error(destpath.to_path_buf(), e))?;
            return Err(SRMRuntimeError::new_src_error(srcpath.to_path_buf(), e));
        }
    }

    if verbose {
        println!("removed '{}'", srcpath.display());
    }

    Ok(())
}

fn move_to_trash_recursive(
    srcpath: &Path,
    destpath: &Path,
    verbose: bool,
) -> Result<(), SRMRuntimeError> {
    if srcpath.is_dir() {
        for entity in srcpath
            .read_dir()
            .map_err(|e| SRMRuntimeError::new_src_error(srcpath.to_path_buf(), e))?
        {
            let entity_path = entity
                .map_err(|e| SRMRuntimeError::new_src_error(srcpath.to_path_buf(), e))?
                .path();

            let dest = entity_path
                .file_name()
                .ok_or_else(|| std::io::Error::from(io::ErrorKind::InvalidData))
                .map_err(|e| SRMRuntimeError::new_src_error(srcpath.to_path_buf(), e))
                .map(|name| destpath.join(name))?;

            if let Err(e) = move_to_trash_recursive(&entity_path, &dest, verbose) {
                eprintln!("{e}");
            }
        }

        if let Err(err) = fs::remove_dir(srcpath)
            .map_err(|e| SRMRuntimeError::new_src_error(srcpath.to_path_buf(), e))
        {
            let _ = fs::remove_dir(destpath);
            return Err(err);
        }
        return Ok(());
    }
    move_to_trash_file(&srcpath, &destpath, verbose)?;
    Ok(())
}

pub fn safe_remove(
    srcpath: &Path,
    trashpath: &Path,
    verbose: bool,
    directory: bool,
    recursive: bool,
) -> Result<PathBuf, SRMRuntimeError> {
    if !srcpath.exists() {
        return Err(SRMRuntimeError::new_src_error(
            srcpath.to_path_buf(),
            io::Error::from(io::ErrorKind::NotFound),
        ));
    }

    let destpath = generate_unique_trash_path(srcpath, trashpath)
        .map_err(|e| SRMRuntimeError::new_src_error(srcpath.to_path_buf(), e))?;

    if srcpath.is_dir() {
        if recursive {
            if let Err(_) = move_to_trash_empty_dir(srcpath, &destpath, verbose) {
                move_to_trash_recursive(srcpath, &destpath, verbose)?;
            }
            return Ok(destpath.to_path_buf());
        }
        if directory {
            let _ = move_to_trash_empty_dir(srcpath, &destpath, verbose)?;
            return Ok(destpath.to_path_buf());
        }
    }
    move_to_trash_file(srcpath, &destpath, verbose)?;
    Ok(destpath.to_path_buf())
}

fn generate_unique_trash_path(srcpath: &Path, trashpath: &Path) -> Result<PathBuf, io::Error> {
    fs::create_dir_all(trashpath)?;

    let srcname = srcpath
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a valid file path")
        })?
        .to_str()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "filename is not valid UTF-8",
            )
        })?;

    let original = trashpath.join(srcname);

    if !original.exists() {
        return Ok(original.to_path_buf());
    }

    let parent = original.parent().unwrap_or_else(|| Path::new(""));

    let mut counter = 1;
    let mut buf = String::with_capacity(srcname.len() + 10);

    loop {
        buf.clear();
        write!(&mut buf, "{}.{}", srcname, counter).unwrap();

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
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    #[test]
    fn test_move_to_trash_for_file() {
        let (_src, _trash) = {
            let src = TempDir::new().unwrap();
            let srcpath = src.path().canonicalize().unwrap();
            let trash = TempDir::new().unwrap();
            let trashpath = trash.path().canonicalize().unwrap();

            let filepath = srcpath.join("file.txt");
            File::create(&filepath).unwrap();
            let file = filepath.canonicalize().unwrap();

            let destpath = trashpath.join("foo.txt");

            assert!(move_to_trash_file(&file, &destpath, false).is_ok());
            assert!(!filepath.exists());
            assert!(trashpath.exists());
            assert!(trashpath.join("foo.txt").exists());
            (src, trash)
        };
    }

    #[test]
    fn test_move_to_trash_for_file_for_different_parentdir_permission() {
        let src = TempDir::new().unwrap();
        let srcpath = src.path();
        let trash = TempDir::new().unwrap();
        let trashpath = trash.path();

        let file = srcpath.join("file.txt");
        File::create(&file).unwrap();

        let dest = trashpath.join("file.txt");

        fs::set_permissions(srcpath, Permissions::from_mode(0o100)).unwrap();
        assert!(move_to_trash_file(&file, &dest, false).is_err());
        fs::set_permissions(srcpath, Permissions::from_mode(0o200)).unwrap();
        assert!(move_to_trash_file(&file, &dest, false).is_err());
        fs::set_permissions(srcpath, Permissions::from_mode(0o400)).unwrap();
        assert!(move_to_trash_file(&file, &dest, false).is_err());
        fs::set_permissions(srcpath, Permissions::from_mode(0o1000)).unwrap();
        assert!(move_to_trash_file(&file, &dest, false).is_err());
        fs::set_permissions(srcpath, Permissions::from_mode(0o500)).unwrap();
        assert!(move_to_trash_file(&file, &dest, false).is_err());
        fs::set_permissions(srcpath, Permissions::from_mode(0o600)).unwrap();
        assert!(move_to_trash_file(&file, &dest, false).is_err());
        fs::set_permissions(srcpath, Permissions::from_mode(0o300)).unwrap();
        assert!(move_to_trash_file(&file, &dest, false).is_ok());
    }

    #[test]
    fn test_move_to_trash_for_empty_dir() {
        let (_src, _trash) = {
            let src = TempDir::new().unwrap();
            let srcpath = src.path().canonicalize().unwrap();
            let trash = TempDir::new().unwrap();
            let trashpath = trash.path().canonicalize().unwrap();

            let empty_dir = srcpath.join("dir1");
            fs::create_dir_all(&empty_dir).unwrap();

            let not_empty_dir = srcpath.join("dir2");
            fs::create_dir_all(&not_empty_dir).unwrap();
            File::create(&not_empty_dir.join("file.txt")).unwrap();

            let dest1 = trashpath.join("dir1");
            let dest2 = trashpath.join("dir1");

            assert!(move_to_trash_empty_dir(&empty_dir, &dest1, false).is_ok());
            assert!(!empty_dir.exists());
            assert!(trashpath.exists());
            assert!(trashpath.join("dir1").exists());

            assert!(move_to_trash_empty_dir(&not_empty_dir, &dest2, false).is_err());
            assert!(not_empty_dir.exists());
            assert!(trashpath.exists());
            assert!(!trashpath.join("dir2").exists());

            (src, trash)
        };
    }

    #[test]
    fn test_move_to_trash_for_empty_dir_for_different_parentdir_permission() {
        let src = TempDir::new().unwrap();
        let srcpath = src.path().canonicalize().unwrap();
        let trash = TempDir::new().unwrap();
        let trash_path = trash.path().canonicalize().unwrap();

        let empty_dir = srcpath.join("dir1");
        fs::create_dir_all(&empty_dir).unwrap();

        let trashpath = trash_path.join("dir1");

        fs::set_permissions(&srcpath, Permissions::from_mode(0o100)).unwrap();
        assert!(move_to_trash_empty_dir(&empty_dir, &trashpath, false).is_err());
        fs::set_permissions(&srcpath, Permissions::from_mode(0o200)).unwrap();
        assert!(move_to_trash_empty_dir(&empty_dir, &trashpath, false).is_err());
        fs::set_permissions(&srcpath, Permissions::from_mode(0o400)).unwrap();
        assert!(move_to_trash_empty_dir(&empty_dir, &trashpath, false).is_err());
        fs::set_permissions(&srcpath, Permissions::from_mode(0o1000)).unwrap();
        assert!(move_to_trash_empty_dir(&empty_dir, &trashpath, false).is_err());
        fs::set_permissions(&srcpath, Permissions::from_mode(0o500)).unwrap();
        assert!(move_to_trash_empty_dir(&empty_dir, &trashpath, false).is_err());
        fs::set_permissions(&srcpath, Permissions::from_mode(0o600)).unwrap();
        assert!(move_to_trash_empty_dir(&empty_dir, &trashpath, false).is_err());
        fs::set_permissions(&srcpath, Permissions::from_mode(0o300)).unwrap();
        assert!(move_to_trash_empty_dir(&empty_dir, &trashpath, false).is_ok());
    }

    #[test]
    fn test_unique_trash_path_generation() {
        let src = TempDir::new().unwrap();
        let srcpath = src.path();
        let trash = TempDir::new().unwrap();
        let trashpath = trash.path();

        assert!(generate_unique_trash_path(srcpath, trashpath).is_ok());
        assert!(generate_unique_trash_path(srcpath, trashpath).is_ok());
        assert!(generate_unique_trash_path(srcpath, trashpath).is_ok());
        assert!(
            !generate_unique_trash_path(srcpath, trashpath)
                .unwrap()
                .exists()
        );
        assert!(
            !generate_unique_trash_path(srcpath, trashpath)
                .unwrap()
                .exists()
        )
    }

    fn print_dir_tree(path: &Path, indent: String) {
        if path.is_dir() {
            // Print the directory name
            println!("{}[DIR] {}", indent, path.display());

            // Read the directory and recursively print contents
            match fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries.filter_map(Result::ok) {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            // Recursively print subdirectory
                            print_dir_tree(&entry_path, format!("{}  ", indent));
                        } else {
                            // Print file name
                            println!("{}  {}", indent, entry_path.display());
                        }
                    }
                }
                Err(e) => {
                    // Handle errors (e.g., permission issues)
                    eprintln!("{}Error reading directory: {}", indent, e);
                }
            }
        } else {
            // If the path is not a directory, print it as a file
            println!("{}[FILE] {}", indent, path.display());
        }
    }

    #[test]
    fn test_move_to_trash_recursive() {
        let (_src, _trash) = {
            let src = tempfile::tempdir().unwrap();
            let src_path = src.path().to_path_buf();

            let trash = tempfile::tempdir().unwrap();
            let trash_path = trash.path().to_path_buf();

            let sub_dir = src_path.join("subdir");
            fs::create_dir_all(&sub_dir).unwrap();
            fs::write(sub_dir.join("file_in_subdir.txt"), "Hello, World!").unwrap();
            fs::write(src_path.join("file1.txt"), "File 1 Content").unwrap();
            fs::write(src_path.join("file2.txt"), "File 2 Content").unwrap();

            print_dir_tree(&src_path, "     ".to_string());

            println!("\n\n\n");
            assert!(safe_remove(&src_path, &trash_path, false, false, true).is_ok());
            assert!(!src_path.join("file1.txt").exists());
            assert!(!src_path.join("file2.txt").exists());

            print_dir_tree(&src_path, "     ".to_string());
            println!("\n\n\n");
            print_dir_tree(&trash_path, "     ".to_string());

            let src_name = src_path.file_name().unwrap();

            assert!(trash_path.exists());
            assert!(trash_path.join(src_name).join("file1.txt").exists());
            assert!(trash_path.join(src_name).join("file2.txt").exists());
            assert!(
                trash_path
                    .join(src_name)
                    .join("subdir")
                    .join("file_in_subdir.txt")
                    .exists()
            );
            (src, trash)
        };
    }

    #[test]
    fn test_move_to_trash_recursive_for_different_permission() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let root = temp_dir.path();

        let file1 = root.join("file1");
        let file2 = root.join("file2");
        let subdir1 = root.join("subdir1");
        let file3 = subdir1.join("file3");
        let file4 = subdir1.join("file4");
        let subdir2 = root.join("subdir2/nested");
        let file5 = subdir2.join("file5");

        File::create(&file1).unwrap();
        File::create(&file2).unwrap();
        fs::create_dir_all(&subdir2).unwrap();
        fs::create_dir(&subdir1).unwrap();
        File::create(&file3).unwrap();
        File::create(&file4).unwrap();
        File::create(&file5).unwrap();

        fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();

        fs::set_permissions(&file1, fs::Permissions::from_mode(0o444)).unwrap(); // read-only
        fs::set_permissions(&file2, fs::Permissions::from_mode(0o644)).unwrap(); // read-write

        fs::set_permissions(&subdir1, fs::Permissions::from_mode(0o555)).unwrap(); // read + exec only
        fs::set_permissions(&file3, fs::Permissions::from_mode(0o444)).unwrap();
        fs::set_permissions(&file4, fs::Permissions::from_mode(0o644)).unwrap();

        fs::set_permissions(&root.join("subdir2"), fs::Permissions::from_mode(0o775)).unwrap();
        fs::set_permissions(&subdir2, fs::Permissions::from_mode(0o775)).unwrap();
        fs::set_permissions(&file5, fs::Permissions::from_mode(0o644)).unwrap();

        let trash = TempDir::new().unwrap();
        let trash_path = trash.path();
        let root_name = root.file_name().unwrap();

        assert!(safe_remove(&root, &trash_path, false, false, true).is_ok());
        assert!(trash_path.join(root_name).exists());
        assert!(trash_path.join(root_name).join("file1").exists());
        assert!(trash_path.join(root_name).join("file2").exists());
        assert!(!trash_path.join(root_name).join("subdir1").exists());
        assert!(!trash_path.join(root_name).join("subdir1/file4").exists());
        assert!(!trash_path.join(root_name).join("subdir1/file5").exists());
        assert!(trash_path.join(root_name).join("subdir2/nested").exists());
        assert!(
            trash_path
                .join(root_name)
                .join("subdir2/nested/file5")
                .exists()
        );
        assert!(trash_path.join(root_name).join("file1").exists());
    }
}
