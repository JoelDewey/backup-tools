use std::io;
use anyhow::{anyhow, bail, Context, Result};
use crossbeam::channel::{after, never, Receiver};
use crossbeam::select;
use nix::libc::pid_t;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::io::{BufRead, BufReader};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tracing::{debug, error, error_span, info, info_span, Span, warn};

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

    let stdout_thread = child
        .stdout
        .take()
        .and_then(|s| read_stream(info_span!("stdout"), s, Box::new(|line| info!("{}", line)))
            .inspect_err(|e| error!(ex=?e, "Failed to open stdout stream."))
            .ok()
        );

    let stderr_thread = child
        .stderr
        .take()
        .and_then(|s| {
            let stream_handle = if stderr_as_stdout {
                read_stream(info_span!("stdout"), s, Box::new(|line| info!("{}", line)))
            } else {
                read_stream(error_span!("stderr"), s, Box::new(|line| error!("{}", line)))
            };
            stream_handle
                .inspect_err(|e| error!(ex=?e, "Failed to open stderr stream."))
                .ok()
        });

    loop {
        let wait_result = child
            .try_wait()
            .map(handle_exit_status)
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
            break;
        }

        let sleep = sleep_duration.map(after).unwrap_or(never());
        select! {
            recv(shutdown_rx) -> _ => {
                warn!("Received notification to shutdown, sending SIGTERM to process.");
                kill(Pid::from_raw(child.id() as pid_t), Signal::SIGTERM)?;
                let exit_status = child.wait().context("Failed to retrieve exit status after sending SIGTERM to child.")?;
                handle_exit_status(Some(exit_status)).unwrap_or(());
                bail!("Killed process due to shutdown.");
            },
            recv(sleep) -> _ => {
                if start.elapsed() <= timeout {
                    debug!(
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
                    handle_exit_status(Some(exit_status)).unwrap_or(());
                    bail!("Killed process due to timeout.");
                }
            }
        }
    }

    if let Some(handle) = stdout_thread {
        handle.join().expect("Could not join to stdout thread.");
    }

    if let Some(handle) = stderr_thread {
        handle.join().expect("Could not join to stderr thread.");
    }

    Ok(())
}

fn handle_exit_status(
    status: Option<ExitStatus>
) -> Option<()> {
    status.map(|exit_status| {
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

fn read_stream<T: 'static + io::Read + Send>(span: Span, stream: T, print_func: Box<dyn Fn(String) + Send>) -> Result<JoinHandle<()>> {
    let name = span.metadata().map(|s| s.name()).unwrap_or_else(|| "Process Thread");
    let buf_reader = BufReader::new(stream);
    std::thread::Builder::new()
        .name(String::from(name))
        .spawn(move || {
            let _enter = span.enter();
            buf_reader
                .lines()
                .map_while(Result::ok)
                .for_each(print_func)
        })
        .context("Failed to start stdout thread.")
}
