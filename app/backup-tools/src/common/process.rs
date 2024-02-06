use anyhow::{anyhow, bail, Context, Result};
use crossbeam::channel::{after, never, Receiver};
use crossbeam::select;
use nix::libc::pid_t;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use tracing::{error, info, instrument, warn};

const WAIT_DURATION_SECS: u64 = 5;
const DEFAULT_TIMEOUT_SECS: u64 = (60 * 2) + 30; // Two minutes and thirty seconds

pub fn create_command(program: &str) -> Command {
    let mut command = Command::new(program);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

pub fn wait_for_child(
    child: Child,
    timeout: Option<Duration>,
    shutdown_rx: &Receiver<()>,
) -> Result<()> {
    wait_for_child_with_redirection(child, timeout, shutdown_rx, false)
}

pub fn wait_for_child_with_redirection(
    mut child: Child,
    timeout: Option<Duration>,
    shutdown_rx: &Receiver<()>,
    stderr_as_stdout: bool,
) -> Result<()> {
    let timeout = timeout.unwrap_or_else(|| Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    let sleep_duration = Some(Duration::from_secs(WAIT_DURATION_SECS));
    let start = Instant::now();

    loop {
        let wait_result = child
            .try_wait()
            .map(|exit_status| handle_exit_status(exit_status, &mut child, stderr_as_stdout))
            .map_err(|e| {
                child
                    .kill()
                    .unwrap_or_else(|kill_error| error!(ex=?kill_error, "Error encountered while sending SIGKILL to child in response to another error."));
                anyhow!(e)
            });

        if wait_result.is_err() {
            return Err(wait_result
                .expect_err("Did not find error in wait_result despite checking for one."));
        }

        let wait_option = wait_result.expect("Did not check wait_result for errors.");

        if wait_option.is_some() {
            return Ok(());
        }

        let sleep = sleep_duration.map(after).unwrap_or(never());
        select! {
            recv(shutdown_rx) -> _ => {
                warn!("Received notification to shutdown, sending SIGTERM to process.");
                kill(Pid::from_raw(child.id() as pid_t), Signal::SIGTERM)?;
            },
            recv(sleep) -> _ => {
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
                    child.kill().context("Failed to send SIGKILL to child.")?;
                    let exit_status = child.wait().context("Failed to retrieve exit status after sending SIGKILL to child.")?;
                    handle_exit_status(Some(exit_status), &mut child, stderr_as_stdout).unwrap_or(());
                    bail!("Killed process due to timeout.");
                }
            }
        }
    }
}

#[instrument(name = "stdout", skip_all)]
fn write_stdout(buffered: Result<String, std::io::Error>) {
    buffered
        .map(|buf| {
            for line in buf.lines() {
                info!("{}", line);
            }
        })
        .map_err(|e| error!(ex=?e, "Failed to read from stdout."))
        .unwrap_or(());
}

#[instrument(name = "stderr", skip_all)]
fn write_stderr(buffered: Result<String, std::io::Error>) {
    buffered
        .map(|buf| {
            for line in buf.lines() {
                error!("{}", line);
            }
        })
        .map_err(|e| error!(ex=?e, "Failed to read from stdout."))
        .unwrap_or(());
}

fn read_buffers_to_logs(child: &mut Child, stderr_to_stdout: bool) {
    let stdout = child.stdout.take().map(|mut stdout| {
        let mut buf = String::new();
        stdout.read_to_string(&mut buf).map(|_| buf)
    });

    if let Some(stdout_result) = stdout {
        write_stdout(stdout_result);
    }

    let stderr = child.stderr.take().map(|mut stderr| {
        let mut buf = String::new();
        stderr.read_to_string(&mut buf).map(|_| buf)
    });

    if let Some(stderr_result) = stderr {
        if stderr_to_stdout {
            write_stdout(stderr_result);
        } else {
            write_stderr(stderr_result);
        }
    }
}

fn handle_exit_status(
    status: Option<ExitStatus>,
    child: &mut Child,
    stderr_to_stdout: bool,
) -> Option<()> {
    status.map(|exit_status| {
        read_buffers_to_logs(child, stderr_to_stdout);

        exit_status
            .code()
            .map(|code| {
                if !exit_status.success() {
                    warn!(
                        exit_code = code,
                        "Process exited with non-success exit code."
                    );
                }
            })
            .unwrap_or_else(|| {
                exit_status
                    .signal()
                    .map(|signal| {
                        warn!(signal = signal, "Process exited due to a signal.");
                    })
                    .unwrap_or_else(|| {
                        warn!("Process completed but its exit code is not known.");
                    })
            })
    })
}
