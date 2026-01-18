//! Task execution engine.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::graph::DependencyGraph;
use crate::package::Package;
use crate::streaming::StreamingTask;

/// Executes tasks across packages respecting dependency order.
pub struct TaskRunner {
    packages_dir: PathBuf,
    graph: DependencyGraph,
    max_parallel: Option<usize>,
}

impl TaskRunner {
    pub fn new(packages_dir: impl Into<PathBuf>, graph: DependencyGraph) -> Self {
        Self {
            packages_dir: packages_dir.into(),
            graph,
            max_parallel: None,
        }
    }

    pub fn with_max_parallel(mut self, max_parallel: Option<usize>) -> Self {
        self.max_parallel = max_parallel;
        self
    }

    pub fn run_task(
        &self,
        task_name: &str,
        package_names: Option<&[String]>,
    ) -> Result<Vec<TaskResult>> {
        // Fast path: single package, no dependencies to worry about
        if let Some(names) = package_names {
            if names.len() == 1 {
                if let Some(package) = self.graph.get_package(&names[0]) {
                    let result = self.execute_task(package, task_name)?;
                    return Ok(vec![result]);
                }
            }
        }

        let packages_to_run = if let Some(names) = package_names {
            names
                .iter()
                .filter_map(|name| self.graph.get_package(name))
                .collect::<Vec<_>>()
        } else {
            self.graph.all_packages()
        };

        // Fast path: empty set
        if packages_to_run.is_empty() {
            return Ok(Vec::new());
        }

        let packages_set: HashSet<String> =
            packages_to_run.iter().map(|p| p.name.clone()).collect();

        let levels = self.graph.dependency_levels();
        let mut results = Vec::new();

        for level in levels {
            let level_packages: Vec<&Package> = level
                .iter()
                .filter(|name| packages_set.contains(*name))
                .filter_map(|name| self.graph.get_package(name))
                .collect();

            if level_packages.is_empty() {
                continue;
            }

            let level_results: Result<Vec<TaskResult>> = level_packages
                .into_par_iter()
                .map(|package| self.execute_task(package, task_name))
                .collect();

            let mut level_results = level_results?;
            results.append(&mut level_results);
        }

        Ok(results)
    }

    pub fn run_task_streaming<F>(
        &self,
        task_name: &str,
        package_names: Option<&[String]>,
        on_output: F,
    ) -> Result<Vec<TaskResult>>
    where
        F: Fn(&str, &str, bool) + Send + Sync,
    {
        let packages_to_run = if let Some(names) = package_names {
            names
                .iter()
                .filter_map(|name| self.graph.get_package(name))
                .collect::<Vec<_>>()
        } else {
            self.graph.all_packages()
        };

        if packages_to_run.is_empty() {
            return Ok(Vec::new());
        }

        let packages_set: HashSet<String> =
            packages_to_run.iter().map(|p| p.name.clone()).collect();

        let levels = self.graph.dependency_levels();
        let mut results = Vec::new();
        use std::sync::{Arc, Mutex};
        let on_output = Arc::new(Mutex::new(on_output));

        for level in levels {
            let level_packages: Vec<&Package> = level
                .iter()
                .filter(|name| packages_set.contains(*name))
                .filter_map(|name| self.graph.get_package(name))
                .collect();

            if level_packages.is_empty() {
                continue;
            }

            let on_output_clone = Arc::clone(&on_output);

            let level_results: Result<Vec<TaskResult>> = level_packages
                .into_par_iter()
                .map(|package| {
                    let package_name = package.name.clone();
                    let package_path = self.packages_dir.join(&package.path);
                    let streaming_task = StreamingTask::spawn(package, task_name, &package_path)?;
                    let stdout = Arc::new(Mutex::new(String::new()));
                    let stderr = Arc::new(Mutex::new(String::new()));
                    let on_output_inner = Arc::clone(&on_output_clone);
                    let stdout_clone = Arc::clone(&stdout);
                    let stderr_clone = Arc::clone(&stderr);
                    let package_name_clone = package_name.clone();

                    let success = streaming_task.stream_output(move |line, is_stderr| {
                        if is_stderr {
                            stderr_clone.lock().unwrap().push_str(line);
                            stderr_clone.lock().unwrap().push('\n');
                            on_output_inner.lock().unwrap()(&package_name_clone, line, true);
                        } else {
                            stdout_clone.lock().unwrap().push_str(line);
                            stdout_clone.lock().unwrap().push('\n');
                            on_output_inner.lock().unwrap()(&package_name_clone, line, false);
                        }
                    })?;

                    Ok(TaskResult {
                        package_name,
                        task_name: task_name.to_string(),
                        success,
                        stdout: Arc::try_unwrap(stdout).unwrap().into_inner().unwrap(),
                        stderr: Arc::try_unwrap(stderr).unwrap().into_inner().unwrap(),
                    })
                })
                .collect();

            results.extend(level_results?);
        }

        Ok(results)
    }

    fn build_task_dependency_order(
        &self,
        package: &Package,
        task_name: &str,
    ) -> Result<Vec<String>> {
        let _task = package
            .get_task(task_name)
            .ok_or_else(|| Error::TaskExecution {
                package: package.name.clone(),
                task: task_name.to_string(),
                message: format!("Task '{}' not found", task_name),
            })?;

        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn visit_task(
            package: &Package,
            task_name: &str,
            order: &mut Vec<String>,
            visited: &mut HashSet<String>,
            visiting: &mut HashSet<String>,
        ) -> Result<()> {
            if visiting.contains(task_name) {
                return Err(Error::TaskExecution {
                    package: package.name.clone(),
                    task: task_name.to_string(),
                    message: format!(
                        "Circular task dependency detected involving '{}'",
                        task_name
                    ),
                });
            }

            if visited.contains(task_name) {
                return Ok(());
            }

            visiting.insert(task_name.to_string());
            let task = package
                .get_task(task_name)
                .ok_or_else(|| Error::TaskExecution {
                    package: package.name.clone(),
                    task: task_name.to_string(),
                    message: format!("Task '{}' not found", task_name),
                })?;

            for dep in &task.depends_on {
                visit_task(package, dep, order, visited, visiting)?;
            }

            visiting.remove(task_name);
            visited.insert(task_name.to_string());
            order.push(task_name.to_string());

            Ok(())
        }

        visit_task(package, task_name, &mut order, &mut visited, &mut visiting)?;

        Ok(order)
    }

    fn execute_task_with_deps(
        &self,
        package: &Package,
        task_name: &str,
    ) -> Result<Vec<TaskResult>> {
        let task_order = self.build_task_dependency_order(package, task_name)?;
        let mut results = Vec::new();

        for task in &task_order {
            let result = self.execute_task_internal(package, task)?;
            results.push(result.clone());
            if !result.success && task == task_name {
                return Ok(results);
            }
        }

        Ok(results)
    }

    fn execute_task_internal(&self, package: &Package, task_name: &str) -> Result<TaskResult> {
        let task = package
            .get_task(task_name)
            .ok_or_else(|| Error::TaskExecution {
                package: package.name.clone(),
                task: task_name.to_string(),
                message: format!(
                    "Task '{}' not found. Available tasks: {}",
                    task_name,
                    package
                        .tasks
                        .iter()
                        .map(|t| t.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })?;

        let package_path = self.packages_dir.join(&package.path);
        let output = Command::new("sh")
            .arg("-c")
            .arg(&task.command)
            .current_dir(&package_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| Error::TaskExecution {
                package: package.name.clone(),
                task: task_name.to_string(),
                message: format!("Failed to execute task: {}", e),
            })?;

        Ok(TaskResult {
            package_name: package.name.clone(),
            task_name: task_name.to_string(),
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn execute_task(&self, package: &Package, task_name: &str) -> Result<TaskResult> {
        let results = self.execute_task_with_deps(package, task_name)?;
        results
            .into_iter()
            .find(|r| r.task_name == task_name)
            .ok_or_else(|| Error::TaskExecution {
                package: package.name.clone(),
                task: task_name.to_string(),
                message: "Task execution failed".to_string(),
            })
    }
}

/// Result of executing a task for a package.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Name of the package that was executed.
    pub package_name: String,
    /// Name of the task that was executed.
    pub task_name: String,
    /// Whether the task succeeded.
    pub success: bool,
    /// Standard output from the task.
    pub stdout: String,
    /// Standard error from the task.
    pub stderr: String,
}
