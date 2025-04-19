use crate::error::SRMCliError;
use crate::error::SRMRuntimeError;
use chrono::Local;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use xdg::BaseDirectories;

pub struct Trash {
    pub files: PathBuf,
    pub info: PathBuf,
}

impl Trash {
    pub fn ensure_trash_dirs() -> Result<Trash, SRMCliError> {
        let xdg_dirs = BaseDirectories::new().map_err(|e| SRMCliError::TrashConfigError(e))?;
        let base = xdg_dirs.get_data_home().join("Trash");

        let files = base.join("files");
        let info = base.join("info");

        fs::create_dir_all(&files).map_err(|e| SRMCliError::TrashDirError(files.clone(), e))?;
        fs::create_dir_all(&info).map_err(|e| SRMCliError::TrashDirError(info.clone(), e))?;

        Ok(Trash { files, info })
    }

    pub fn create_trash_info_file(
        &self,
        srcpath: PathBuf,
        destpath: &Path,
    ) -> Result<(), SRMRuntimeError> {
        let trashinfo_path = self.info.join(format!(
            "{}.trashinfo",
            destpath.file_name().unwrap().to_string_lossy()
        ));

        let original_path = &srcpath;
        let deletion_date = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        let info_contents = format!(
            "[Trash Info]\nPath={}\nDeletionDate={}\n",
            original_path.display(),
            deletion_date
        );

        fs::write(&trashinfo_path, info_contents)
            .map_err(|e| SRMRuntimeError::new_trashinfo_error(srcpath, e))?;
        Ok(())
    }
}
