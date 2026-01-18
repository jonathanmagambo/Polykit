//! Streaming output utilities for task execution.

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::command_validator::CommandValidator;
use crate::error::{Error, Result};
use crate::package::Package;

pub struct StreamingTask {
    child: Child,
    package_name: String,
    task_name: String,
}

impl StreamingTask {
    pub async fn spawn(
        package: &Package,
        task_name: &str,
        package_path: &std::path::Path,
    ) -> Result<Self> {
        let task = package
            .get_task(task_name)
            .ok_or_else(|| Error::TaskExecution {
                package: package.name.clone(),
                task: task_name.to_string(),
                message: format!("Task '{}' not found", task_name),
            })?;

        let validator = CommandValidator::new();
        validator.validate(&task.command)?;

        let child = Command::new("sh")
            .arg("-c")
            .arg(&task.command)
            .current_dir(package_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| Error::TaskExecution {
                package: package.name.clone(),
                task: task_name.to_string(),
                message: format!("Failed to spawn task: {}", e),
            })?;

        Ok(Self {
            child,
            package_name: package.name.clone(),
            task_name: task_name.to_string(),
        })
    }

    pub async fn stream_output<F>(mut self, mut on_line: F) -> Result<bool>
    where
        F: FnMut(&str, bool) + Send,
    {
        let package_name = self.package_name.clone();
        let task_name = self.task_name.clone();
        let mut stdout = self
            .child
            .stdout
            .take()
            .ok_or_else(|| Error::TaskExecution {
                package: package_name.clone(),
                task: task_name.clone(),
                message: "Failed to capture stdout".to_string(),
            })?;
        let mut stderr = self
            .child
            .stderr
            .take()
            .ok_or_else(|| Error::TaskExecution {
                package: package_name.clone(),
                task: task_name.clone(),
                message: "Failed to capture stderr".to_string(),
            })?;

        let mut stdout_reader = BufReader::new(&mut stdout);
        let mut stderr_reader = BufReader::new(&mut stderr);
        let mut stdout_done = false;
        let mut stderr_done = false;
        let mut exit_status = None;

        loop {
            let mut stdout_line = String::new();
            let mut stderr_line = String::new();

            tokio::select! {
                result = stdout_reader.read_line(&mut stdout_line), if !stdout_done => {
                    match result {
                        Ok(0) => {
                            stdout_done = true;
                        }
                        Ok(_) => {
                            let trimmed = stdout_line.trim_end();
                            if !trimmed.is_empty() {
                                on_line(trimmed, false);
                            }
                        }
                        Err(e) => {
                            return Err(Error::TaskExecution {
                                package: self.package_name.clone(),
                                task: self.task_name.clone(),
                                message: format!("Failed to read stdout: {}", e),
                            });
                        }
                    }
                }
                result = stderr_reader.read_line(&mut stderr_line), if !stderr_done => {
                    match result {
                        Ok(0) => {
                            stderr_done = true;
                        }
                        Ok(_) => {
                            let trimmed = stderr_line.trim_end();
                            if !trimmed.is_empty() {
                                on_line(trimmed, true);
                            }
                        }
                        Err(e) => {
                            return Err(Error::TaskExecution {
                                package: self.package_name.clone(),
                                task: self.task_name.clone(),
                                message: format!("Failed to read stderr: {}", e),
                            });
                        }
                    }
                }
                status = self.child.wait(), if exit_status.is_none() => {
                    exit_status = Some(status.map_err(|e| Error::TaskExecution {
                        package: self.package_name.clone(),
                        task: self.task_name.clone(),
                        message: format!("Failed to wait for process: {}", e),
                    })?);
                }
            }

            // If process finished and both streams are done, exit
            if let Some(status) = exit_status {
                if stdout_done && stderr_done {
                    return Ok(status.success());
                }
            }
        }
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn task_name(&self) -> &str {
        &self.task_name
    }
}
