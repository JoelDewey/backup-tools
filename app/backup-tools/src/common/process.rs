use anyhow::{anyhow, bail, Context, Result};
use crossbeam::channel::{after, bounded, never, Receiver};
use crossbeam::select;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use subprocess::{Communicator, ExitStatus, Popen};
use tracing::{debug, error, info, trace, warn};

const WAIT_DURATION_SECS: u64 = 5;
const DEFAULT_TIMEOUT_SECS: u64 = (60 * 2) + 30; // Two minutes and thirty seconds

pub fn wait_for_subprocess(
    mut process: Popen,
    timeout: Option<Duration>,
    shutdown_rx: &Receiver<()>,
) -> Result<()> {
    let timeout = timeout.unwrap_or_else(|| Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    let wait_duration = Duration::from_secs(WAIT_DURATION_SECS);
    let (process_complete_tx, process_complete_rx) = bounded(1);
    let handle = read_from_communicator(
        process.communicate_start(None),
        Duration::from_secs(1),
        process_complete_rx,
    )
    .map_err(|e| error!(ex=?e, "Failed to start communicator thread."))
    .ok();
    let duration = Some(wait_duration);
    let start = Instant::now();

    while process.poll().is_none() {
        let wait = duration.map(|d| after(d)).unwrap_or(never());

        select! {
            recv(shutdown_rx) -> _ => {
                let _ = process_complete_tx
                    .send_timeout((), Duration::from_secs(10))
                    .is_err_and(|e| {
                        error!(ex=?e, "Failed to communicate process shutdown to communicator thread.");
                        true
                    });
                process.terminate()?;
            },
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
                    let _ = process_complete_tx
                        .send_timeout((), Duration::from_secs(10))
                        .is_err_and(|e| {
                            error!(ex=?e, "Failed to communicate process shutdown to communicator thread.");
                            true
                        });
                    process.kill()?;
                    bail!("Killed process due to timeout.");
                }
            }
        }
    }

    process_complete_tx.send_timeout((), Duration::from_secs(10))?;
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
    time_limit: Duration,
    process_complete: Receiver<()>,
) -> Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(String::from("Process Communicator"))
        .spawn(move || {
            let mut c = communicator;
            let t = time_limit;

            loop {
                let mut is_stdout_empty = false;
                let mut is_stderr_empty = false;
                c = c.limit_time(t.clone());

                if let Ok(tuple) = c.read_string() {
                    if let Some(stdout) = tuple.0 {
                        if stdout.is_empty() {
                            trace!("Empty stdout.");
                            is_stdout_empty = true;
                        } else {
                            info!("Process stdout: {}", stdout);
                        }
                    } else {
                        // stdout was not piped so it'll always be empty.
                        is_stdout_empty = true;
                    }

                    if let Some(stderr) = tuple.1 {
                        if stderr.is_empty() {
                            trace!("Empty stderr.");
                            is_stderr_empty = true;
                        } else {
                            error!("Process stderr: {}", stderr);
                        }
                    } else {
                        // stderr was not piped, so it'll always be empty.
                        is_stderr_empty = true;
                    }
                } else {
                    // Assume that both streams were not piped.
                    is_stdout_empty = true;
                    is_stderr_empty = true;
                }

                if is_stdout_empty && is_stderr_empty && process_complete.is_full() {
                    break;
                }
            }

            debug!("Loop ended.");
        })
        .context("Failed to start communicator thread.")
}
