use std::fs;
use std::path::{Path, PathBuf};

pub fn create_invalid_pdf_qpdf_probe(dir: &Path, json_marker: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let script_path = dir.join("qpdf.cmd");
        fs::write(
            &script_path,
            format!(
                "@echo off\r\nif \"%1\"==\"--show-npages\" (\r\n  >&2 echo invalid pdf\r\n  exit /b 2\r\n)\r\nif \"%1\"==\"--json\" (\r\n  type nul > \"{}\"\r\n  echo {{}}\r\n  exit /b 0\r\n)\r\nexit /b 1\r\n",
                json_marker.display()
            ),
        )
        .expect("probe script should be created");
        script_path
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let script_path = dir.join("qpdf");
        fs::write(
            &script_path,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"--show-npages\" ]; then\n  echo invalid pdf >&2\n  exit 2\nfi\nif [ \"$1\" = \"--json\" ]; then\n  : > '{}'\n  echo '{{}}'\n  exit 0\nfi\nexit 1\n",
                json_marker.display()
            ),
        )
        .expect("probe script should be created");
        let mut permissions = fs::metadata(&script_path)
            .expect("probe script should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("probe script should be executable");
        script_path
    }
}

pub fn create_extract_probe_with_log(
    dir: &Path,
    total_pages: u32,
    log_path: &Path,
    succeed: bool,
) -> PathBuf {
    #[cfg(windows)]
    {
        let script_path = dir.join("qpdf.cmd");
        let script = if succeed {
            format!(
                "@echo off\r\nif \"%1\"==\"--show-npages\" (\r\n  echo {total_pages}\r\n  exit /b 0\r\n)\r\nif \"%2\"==\"--pages\" (\r\n  >>\"{}\" echo %4\r\n  >\"%6\" echo pdf\r\n  exit /b 0\r\n)\r\n>&2 echo unexpected call\r\nexit /b 1\r\n",
                log_path.display()
            )
        } else {
            "@echo off\r\n>&2 echo extract failure\r\nexit /b 2\r\n".to_string()
        };
        fs::write(&script_path, script).expect("probe script should be created");
        script_path
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let script_path = dir.join("qpdf");
        let script = if succeed {
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"--show-npages\" ]; then\n  echo {total_pages}\n  exit 0\nfi\nif [ \"$2\" = \"--pages\" ]; then\n  echo \"$4\" >> '{}'\n  printf 'pdf' > \"$6\"\n  exit 0\nfi\necho unexpected call >&2\nexit 1\n",
                log_path.display()
            )
        } else {
            "#!/bin/sh\necho extract failure >&2\nexit 2\n".to_string()
        };
        fs::write(&script_path, script).expect("probe script should be created");
        let mut permissions = fs::metadata(&script_path)
            .expect("probe script should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions)
            .expect("probe script should be executable");
        script_path
    }
}

pub fn create_pdftoppm_probe(
    dir: &Path,
    succeed: bool,
    skip_output: bool,
    log_path: Option<&Path>,
) -> PathBuf {
    #[cfg(windows)]
    {
        let script_path = dir.join("pdftoppm.cmd");
        let logging = if let Some(log) = log_path {
            format!(">>\"{}\" echo %2,%4,%6\r\n", log.display())
        } else {
            String::new()
        };
        let script = if succeed {
            format!(
                "@echo off\r\n{logging}if \"%1\"==\"-f\" (\r\n  if not \"{}\"==\"true\" (\r\n    >\"%9-1.png\" echo png\r\n  )\r\n  exit /b 0\r\n)\r\n>&2 echo unexpected call\r\nexit /b 1\r\n",
                if skip_output { "true" } else { "false" }
            )
        } else {
            "@echo off\r\n>&2 echo render failure\r\nexit /b 2\r\n".to_string()
        };
        fs::write(&script_path, script).expect("probe script should be created");
        script_path
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let script_path = dir.join("pdftoppm");
        let logging = if let Some(log) = log_path {
            format!("echo \"$2,$4,$6\" >> '{}'\n", log.display())
        } else {
            String::new()
        };
        let script = if succeed {
            format!(
                "#!/bin/sh\n{logging}if [ \"$1\" = \"-f\" ]; then\n  if [ \"{}\" != \"true\" ]; then\n    printf 'png' > \"$9-1.png\"\n  fi\n  exit 0\nfi\necho unexpected call >&2\nexit 1\n",
                if skip_output { "true" } else { "false" }
            )
        } else {
            "#!/bin/sh\necho render failure >&2\nexit 2\n".to_string()
        };
        fs::write(&script_path, script).expect("probe script should be created");
        let mut permissions = fs::metadata(&script_path)
            .expect("probe script should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions)
            .expect("probe script should be executable");
        script_path
    }
}
