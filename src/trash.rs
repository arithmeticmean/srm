use crate::error::SRMError;
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
    pub fn ensure_trash_dirs() -> Result<Trash, SRMError> {
        let xdg_dirs = BaseDirectories::new().map_err(|e| SRMError::TrashConfigError(e))?;
        let base = xdg_dirs.get_data_home().join("Trash");

        let files = base.join("files");
        let info = base.join("info");

        fs::create_dir_all(&files).map_err(|e| SRMError::TrashDirError(files.clone(), e))?;
        fs::create_dir_all(&info).map_err(|e| SRMError::TrashDirError(info.clone(), e))?;

        Ok(Trash { files, info })
    }

    pub fn create_trash_info_file(
        &self,
        srcpath: &Path,
        destpath: &Path,
    ) -> Result<(), SRMRuntimeError> {
        let trashinfo_path = self.info.join(format!(
            "{}.trashinfo",
            destpath.file_name().unwrap().to_string_lossy()
        ));

        let original_path = srcpath;
        let deletion_date = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        let info_contents = format!(
            "[Trash Info]\nPath={}\nDeletionDate={}\n",
            original_path.display(),
            deletion_date
        );

        fs::write(&trashinfo_path, info_contents)
            .map_err(|e| SRMRuntimeError::TrashInfoError(trashinfo_path, e))
    }
}
