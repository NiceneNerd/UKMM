//! Process and file-manager interactions: opening folders/URLs in the OS
//! shell, launching the emulator, and restarting the app after a self-update.
//! None of this depends on a GUI toolkit; frontends dispatch the matching
//! [`Command`](crate::session::Command)s (or call the helpers directly for
//! plain links, which change no session state).

use std::{path::Path, time::Duration};

use anyhow_ext::{Context, Result};

/// Open a path (file or folder) with the OS default handler. Failures are
/// ignored, matching the previous `open::that(..).unwrap_or(())` behaviour.
pub fn open_path(path: impl AsRef<Path>) {
    let _ = open::that(path.as_ref());
}

/// Open a URL in the default browser. Failures are ignored.
pub fn open_url(url: &str) {
    let _ = open::that(url);
}

/// Launch the emulator from its configured command string. Runs the child for
/// about a second and reports a non-zero exit as an error (a crashed emulator
/// should be visible), but a still-running emulator is fine. Intended to run
/// on a worker thread, never inside a render closure.
pub fn launch_emulator(command: &str) -> Result<()> {
    let mut iter = shlex::split(&command.replace('\\', "/"))
        .unwrap_or_default()
        .into_iter();
    let (cmd, args) = (
        iter.next().context("Empty emulator command")?,
        iter.collect::<Vec<_>>(),
    );
    let mut child = std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .context("Failed to open emulator")?;
    std::thread::sleep(Duration::from_secs(1));
    match child.try_wait() {
        Ok(Some(status)) => {
            if !status.success() {
                anyhow_ext::bail!("Emulator exited with non-zero exit status {status}");
            }
        }
        Ok(None) => {}
        Err(e) => {
            return Err(
                anyhow_ext::Error::from(e).context("There was an error running the emulator")
            );
        }
    }
    Ok(())
}

/// Restart the app: spawn the current executable (a `.bak` extension from the
/// self-update is stripped) in its own process group on unix. The frontend
/// closes its window after calling this.
pub fn restart() -> Result<()> {
    let mut exe = std::env::current_exe()?;
    if exe
        .extension()
        .and_then(|x| x.to_str())
        .is_some_and(|ext| ext.contains("bak"))
    {
        exe.set_extension("");
    }
    let mut command = std::process::Command::new(exe);
    #[cfg(unix)]
    {
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
    }
    command.spawn().context("Failed to restart UKMM")?;
    Ok(())
}
