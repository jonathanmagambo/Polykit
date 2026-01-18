//! Streaming output utilities for task execution.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use crate::error::{Error, Result};
use crate::package::Package;

pub struct StreamingTask {
    child: Child,
    package_name: String,
    task_name: String,
}

impl StreamingTask {
    pub fn spawn(
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

        let child = Command::new("sh")
            .arg("-c")
            .arg(&task.command)
            .current_dir(package_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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

    pub fn stream_output<F>(mut self, mut on_line: F) -> Result<bool>
    where
        F: FnMut(&str, bool),
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

        loop {
            let mut line = String::new();
            let mut has_output = false;

            if stdout_reader.read_line(&mut line).unwrap_or(0) > 0 {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    on_line(trimmed, false);
                    has_output = true;
                }
                line.clear();
            }

            if stderr_reader.read_line(&mut line).unwrap_or(0) > 0 {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    on_line(trimmed, true);
                    has_output = true;
                }
                line.clear();
            }

            if !has_output {
                let package_name = self.package_name.clone();
                let task_name = self.task_name.clone();
                if self
                    .child
                    .try_wait()
                    .map_err(|e| Error::TaskExecution {
                        package: package_name.clone(),
                        task: task_name.clone(),
                        message: format!("Failed to check process: {}", e),
                    })?
                    .is_some()
                {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        let package_name = self.package_name.clone();
        let task_name = self.task_name.clone();
        let status = self.child.wait().map_err(|e| Error::TaskExecution {
            package: package_name,
            task: task_name,
            message: format!("Failed to wait for process: {}", e),
        })?;

        Ok(status.success())
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn task_name(&self) -> &str {
        &self.task_name
    }
}
