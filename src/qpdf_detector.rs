use std::env;
use std::ffi::OsString;
#[cfg(any(test, unix))]
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;

const ENV_QPDF: &str = "NATA_QPDF";

#[derive(Debug, Clone)]
pub struct QpdfLocator<'a> {
    pub cli_path: Option<&'a Path>,
    pub env_path: Option<OsString>,
    pub path_env: Option<OsString>,
}

impl<'a> QpdfLocator<'a> {
    pub fn from_system(cli_path: Option<&'a Path>) -> Self {
        Self {
            cli_path,
            env_path: env::var_os(ENV_QPDF),
            path_env: env::var_os("PATH"),
        }
    }
}

pub fn detect(cli_path: Option<&Path>) -> Result<PathBuf, AppError> {
    detect_with(QpdfLocator::from_system(cli_path))
}

pub fn detect_with(locator: QpdfLocator<'_>) -> Result<PathBuf, AppError> {
    if let Some(path) = locator.cli_path {
        return validate_candidate(path.to_path_buf());
    }

    if let Some(path) = locator.env_path {
        return validate_candidate(PathBuf::from(path));
    }

    if let Some(found) = find_in_path(locator.path_env.as_ref()) {
        return validate_candidate(found);
    }

    Err(AppError::QpdfNotFound)
}

fn validate_candidate(path: PathBuf) -> Result<PathBuf, AppError> {
    if is_executable_file(&path) {
        Ok(path)
    } else {
        Err(AppError::QpdfNotExecutable { path })
    }
}

fn find_in_path(path_env: Option<&OsString>) -> Option<PathBuf> {
    let candidates = executable_names();
    let path_env = path_env?;

    env::split_paths(path_env).find_map(|dir| {
        candidates
            .iter()
            .map(|name| dir.join(name))
            .find(|candidate| is_executable_file(candidate))
    })
}

fn executable_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["qpdf.exe", "qpdf"]
    } else {
        &["qpdf"]
    }
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        return fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }

    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{detect_with, QpdfLocator};

    #[test]
    fn cli_path_has_highest_priority() {
        let cli_file = create_test_file("cli");
        let locator = QpdfLocator {
            cli_path: Some(cli_file.as_path()),
            env_path: Some(OsString::from("missing-from-env")),
            path_env: Some(OsString::from("missing-from-path")),
        };

        let detected = detect_with(locator).expect("cli path should win");
        assert_eq!(detected, cli_file);
    }

    #[test]
    fn env_path_is_used_when_cli_path_is_absent() {
        let env_file = create_test_file("env");
        let locator = QpdfLocator {
            cli_path: None,
            env_path: Some(OsString::from(env_file.as_os_str())),
            path_env: Some(OsString::from("missing-from-path")),
        };

        let detected = detect_with(locator).expect("env path should be used");
        assert_eq!(detected, env_file);
    }

    #[test]
    fn path_lookup_is_used_last() {
        let path_file = create_path_candidate();
        let path_dir = path_file.parent().expect("candidate should have parent");
        let locator = QpdfLocator {
            cli_path: None,
            env_path: None,
            path_env: Some(OsString::from(path_dir.as_os_str())),
        };

        let detected = detect_with(locator).expect("path lookup should find qpdf candidate");
        assert_eq!(detected, path_file);
    }

    #[cfg(unix)]
    #[test]
    fn path_lookup_skips_non_executable_files() {
        use std::os::unix::fs::PermissionsExt;

        let dir = unique_test_dir("non-executable");
        fs::create_dir_all(&dir).expect("should create temp dir");
        let non_executable = dir.join("qpdf");
        fs::write(&non_executable, b"test").expect("should create qpdf candidate");
        let mut permissions = fs::metadata(&non_executable)
            .expect("candidate should exist")
            .permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&non_executable, permissions)
            .expect("should update permissions");

        let executable_dir = unique_test_dir("executable");
        fs::create_dir_all(&executable_dir).expect("should create temp dir");
        let executable = executable_dir.join("qpdf");
        fs::write(&executable, b"test").expect("should create qpdf candidate");
        let mut permissions = fs::metadata(&executable)
            .expect("candidate should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("should update permissions");

        let path_env = env::join_paths([dir.as_path(), executable_dir.as_path()])
            .expect("path env should be constructible");
        let locator = QpdfLocator {
            cli_path: None,
            env_path: None,
            path_env: Some(path_env),
        };

        let detected = detect_with(locator).expect("path lookup should skip non executable file");
        assert_eq!(detected, executable);
    }

    fn create_path_candidate() -> PathBuf {
        let dir = unique_test_dir("path");
        fs::create_dir_all(&dir).expect("should create temp dir");
        let file = dir.join(if cfg!(windows) { "qpdf.exe" } else { "qpdf" });
        fs::write(&file, b"test").expect("should create qpdf candidate");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&file)
                .expect("candidate should exist")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&file, permissions).expect("should update permissions");
        }

        file
    }

    fn create_test_file(label: &str) -> PathBuf {
        let file = unique_test_dir(label).with_extension("bin");
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).expect("should create temp dir");
        }
        fs::write(&file, b"test").expect("should create temp file");
        file
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("nata-{label}-{nonce}"))
    }
}
