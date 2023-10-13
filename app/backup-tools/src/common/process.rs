use anyhow::{anyhow, bail, Result};
use crossbeam::channel::{after, never, Receiver};
use crossbeam::select;
use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};
use subprocess::{ExitStatus, Popen};
use tracing::{error, info, warn};

const WAIT_DURATION_SECS: u64 = 5;
const DEFAULT_TIMEOUT_SECS: u64 = (60 * 2) + 30; // Two minutes and thirty seconds

pub fn wait_for_subprocess(
    mut process: Popen,
    timeout: Option<Duration>,
    shutdown_rx: &Receiver<()>,
) -> Result<()> {
    let timeout = timeout.unwrap_or_else(|| Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    let duration = Some(Duration::from_secs(WAIT_DURATION_SECS));
    let start = Instant::now();

    while process.poll().is_none() {
        let wait = duration.map(|d| after(d)).unwrap_or(never());

        select! {
            recv(shutdown_rx) -> _ => process.terminate()?,
            recv(wait) -> _ => {
                if start.elapsed() <= timeout {
                    info!(
                        time_elapsed_seconds=&start.elapsed().as_secs(),
                        "Process still running, waiting another {} seconds.",
                        WAIT_DURATION_SECS
                    );
                } else {
                    warn!(
                        time_elapsed_seconds=&start.elapsed().as_secs(),
                        timeout_seconds=timeout.as_secs(),
                        "Reached timeout while waiting for process completion, killing the process."
                    );
                    process.kill()?;
                    bail!("Killed process due to timeout.");
                }
            }
        }
    }

    let p_output = get_stream(&process.stdout);
    if !p_output.is_empty() {
        info!("Process stdout: \r{}", p_output);
    }

    let p_err = get_stream(&process.stderr);
    if !p_err.is_empty() {
        error!("Process stderr: \r{}", p_err);
    }

    let exit_status = process
        .exit_status()
        .ok_or_else(|| anyhow!("Failed to retrieve exit status from process."))?;
    if exit_status.success() {
        Ok(())
    } else {
        match exit_status {
            ExitStatus::Exited(code) => {
                warn!(
                    exit_code = code,
                    "Process exited with non-success exit code."
                );
            }
            ExitStatus::Signaled(signal) => {
                warn!(signal = signal, "Process exited due to a signal.")
            }
            ExitStatus::Other(val) => {
                warn!(
                    value = val,
                    "Unexpected exit status that cannot be described."
                )
            }
            ExitStatus::Undetermined => {
                warn!("Process completed successfully but its exit code is not known.")
            }
        }

        Err(anyhow!("Process did not complete successfully."))
    }
}

fn get_stream(stream: &Option<File>) -> String {
    stream
        .as_ref()
        .and_then(|mut f| {
            let mut output = String::new();
            match f.read_to_string(&mut output) {
                Ok(_) => Some(output),
                Err(e) => {
                    error!(ex=%e, "Error while reading process stream.");
                    None
                }
            }
        })
        .unwrap_or_else(|| String::from("No stdout recorded."))
}
