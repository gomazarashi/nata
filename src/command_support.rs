use std::path::Path;
use std::process::Command;

pub fn new_command(executable: &Path) -> Command {
    #[cfg(windows)]
    {
        if is_windows_batch_wrapper(executable) {
            let mut command = Command::new("cmd.exe");
            command.arg("/C").arg(executable);
            return command;
        }
    }

    Command::new(executable)
}

#[cfg(windows)]
fn is_windows_batch_wrapper(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "cmd" | "bat"))
}

pub fn stderr_summary(binary: &str, stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).trim().to_string();
    if text.is_empty() {
        format!("{binary} did not provide error details")
    } else {
        text
    }
}
