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
