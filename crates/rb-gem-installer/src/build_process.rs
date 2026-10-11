use crate::{
    BuildEvent, BuildStage, OutputStream,
    constants::{
        BUILD_CHANNEL_CAPACITY, BUILD_POLL_INTERVAL, MAX_BUILD_OUTPUT_BYTES, READ_BUFFER_BYTES,
    },
};
use anyhow::{Context, Result, ensure};
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub(crate) struct ProcessOutput {
    pub(crate) stdout: Vec<u8>,
}

fn append(tail: &mut Vec<u8>, bytes: &[u8]) {
    tail.extend_from_slice(bytes);
    if tail.len() > MAX_BUILD_OUTPUT_BYTES {
        tail.drain(..tail.len() - MAX_BUILD_OUTPUT_BYTES);
    }
}

fn terminate(child: &mut Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = rustix::process::Pid::from_raw(child.id() as i32) {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

struct RunningChild {
    child: Child,
    finished: bool,
}

impl std::ops::Deref for RunningChild {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.child
    }
}

impl std::ops::DerefMut for RunningChild {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}

impl Drop for RunningChild {
    fn drop(&mut self) {
        if !self.finished {
            terminate(&mut self.child);
        }
    }
}

pub(crate) fn run(
    mut command: Command,
    extension: &Path,
    stage: BuildStage,
    timeout: Duration,
    events: &mut impl FnMut(BuildEvent),
) -> Result<ProcessOutput> {
    let program = command.get_program().to_os_string();
    let arguments = command.get_args().map(|arg| arg.to_os_string()).collect();
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let started = Instant::now();
    events(BuildEvent::Started {
        extension: extension.to_path_buf(),
        stage,
        program: program.clone(),
        arguments,
    });
    let child = command
        .spawn()
        .with_context(|| format!("Cannot start build tool {}", program.to_string_lossy()))?;
    let mut child = RunningChild {
        child,
        finished: false,
    };
    let stdout = child.stdout.take().context("Missing child stdout")?;
    let stderr = child.stderr.take().context("Missing child stderr")?;
    let (sender, receiver) = mpsc::sync_channel(BUILD_CHANNEL_CAPACITY);
    let streams: [(OutputStream, Box<dyn Read + Send>); 2] = [
        (OutputStream::Stdout, Box::new(stdout)),
        (OutputStream::Stderr, Box::new(stderr)),
    ];
    let readers: Vec<_> = streams
        .into_iter()
        .map(|(stream, mut reader)| {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut bytes = [0; READ_BUFFER_BYTES];
                loop {
                    match reader.read(&mut bytes) {
                        Ok(0) => break,
                        Ok(size) => {
                            if sender.send((stream, Ok(bytes[..size].to_vec()))).is_err() {
                                break;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(error) => {
                            let _ = sender.send((stream, Err(error)));
                            break;
                        }
                    }
                }
            })
        })
        .collect();
    drop(sender);
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let result = (|| -> Result<ExitStatus> {
        let mut status = None;
        loop {
            if started.elapsed() >= timeout {
                anyhow::bail!(
                    "Build tool {} timed out after {:.1}s",
                    program.to_string_lossy(),
                    timeout.as_secs_f64()
                );
            }
            match receiver.recv_timeout(BUILD_POLL_INTERVAL) {
                Ok((stream, bytes)) => {
                    let bytes = bytes?;
                    append(
                        if stream == OutputStream::Stdout {
                            &mut output
                        } else {
                            &mut errors
                        },
                        &bytes,
                    );
                    events(BuildEvent::Output {
                        extension: extension.to_path_buf(),
                        stage,
                        stream,
                        bytes,
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if status.is_none() {
                status = child.try_wait()?;
            }
        }
        loop {
            if let Some(status) = status {
                return Ok(status);
            }
            if started.elapsed() >= timeout {
                anyhow::bail!("Build tool {} timed out", program.to_string_lossy());
            }
            status = child.try_wait()?;
            if status.is_none() {
                thread::sleep(BUILD_POLL_INTERVAL);
            }
        }
    })();
    if result.is_err() {
        terminate(&mut child);
    }
    child.finished = true;
    drop(receiver);
    for reader in readers {
        let _ = reader.join();
    }
    let status = result.with_context(|| {
        format!(
            "{}\n{}",
            String::from_utf8_lossy(&output),
            String::from_utf8_lossy(&errors)
        )
    })?;
    ensure!(
        status.success(),
        "Build tool {} failed ({status})\n{}\n{}",
        program.to_string_lossy(),
        String::from_utf8_lossy(&output),
        String::from_utf8_lossy(&errors)
    );
    events(BuildEvent::Finished {
        extension: extension.to_path_buf(),
        stage,
        elapsed: started.elapsed(),
    });
    Ok(ProcessOutput { stdout: output })
}
