use anyhow::{anyhow, bail, Context, Result};
use crossbeam::channel::{after, never, Receiver};
use crossbeam::select;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use subprocess::{Communicator, ExitStatus, Popen};
use tracing::{debug, error, info, trace, warn};

const WAIT_DURATION_SECS: u64 = 5;
const DEFAULT_TIMEOUT_SECS: u64 = (60 * 2) + 30; // Two minutes and thirty seconds
const TIMEOUT_BUFFER: Duration = Duration::from_secs(5 * 60); // Five minutes

pub fn wait_for_subprocess(
    mut process: Popen,
    timeout: Option<Duration>,
    shutdown_rx: &Receiver<()>,
) -> Result<()> {
    let timeout = timeout.unwrap_or_else(|| Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    let wait_duration = Duration::from_secs(WAIT_DURATION_SECS);
    let handle = read_from_communicator(
        process.communicate_start(None),
        timeout.saturating_add(TIMEOUT_BUFFER)
    )
        .map_err(|e| error!(ex=?e, "Failed to start communicator thread."))
        .ok();
    let duration = Some(wait_duration);
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

    if let Some(h) = handle {
        h.join().expect("Couldn't join to the communicator thread.");
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

fn read_from_communicator(
    communicator: Communicator,
    timeout: Duration
) -> Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(String::from("Process Communicator"))
        .spawn(move || {
            let mut c = communicator.limit_time(timeout);

            match c.read_string() {
                Ok(tuple) => {
                    if let Some(stdout) = tuple.0 {
                        if stdout.is_empty() {
                            trace!("Empty stdout.");
                        } else {
                            info!("Process stdout: {}", stdout);
                        }
                    }

                    if let Some(stderr) = tuple.1 {
                        if stderr.is_empty() {
                            trace!("Empty stderr.");
                        } else {
                            error!("Process stderr: {}", stderr);
                        }
                    }
                }
                Err(e) => { error!(ex=?e, "Failed to read from stdout/stderr."); }
            }

            debug!("Communicator ended.");
        })
        .context("Failed to start communicator thread.")
}
