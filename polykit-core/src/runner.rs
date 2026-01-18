//! Task execution engine and orchestration.

use std::collections::HashSet;
use std::path::PathBuf;

use rayon::prelude::*;
use crossbeam::channel;

use std::sync::Arc;

use crate::command_validator::CommandValidator;
use crate::error::{Error, Result};
use crate::executor::TaskExecutor;
use crate::graph::DependencyGraph;
use crate::package::Package;
use crate::remote_cache::RemoteCache;
use crate::streaming::StreamingTask;
use crate::task_cache::TaskCache;

/// Executes tasks across packages respecting dependency order.
pub struct TaskRunner {
    packages_dir: PathBuf,
    graph: DependencyGraph,
    max_parallel: Option<usize>,
    command_validator: CommandValidator,
    task_cache: Option<TaskCache>,
    remote_cache: Option<Arc<RemoteCache>>,
    thread_pool: Arc<rayon::ThreadPool>,
    executor: TaskExecutor,
}

impl TaskRunner {
    pub fn new(packages_dir: impl Into<PathBuf>, graph: DependencyGraph) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(rayon::current_num_threads())
            .thread_name(|i| format!("polykit-worker-{}", i))
            .build()
            .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

        let packages_dir_path = packages_dir.into();
        let executor = TaskExecutor::new(
            packages_dir_path.clone(),
            graph.clone(),
            CommandValidator::new(),
            None,
            None,
        );

        Self {
            packages_dir: packages_dir_path,
            graph,
            max_parallel: None,
            command_validator: CommandValidator::new(),
            task_cache: None,
            remote_cache: None,
            thread_pool: Arc::new(pool),
            executor,
        }
    }

    pub fn with_command_validator(mut self, validator: CommandValidator) -> Self {
        self.command_validator = validator.clone();
        self.executor = TaskExecutor::new(
            self.packages_dir.clone(),
            self.graph.clone(),
            validator,
            self.task_cache.clone(),
            self.remote_cache.clone(),
        );
        self
    }

    pub fn with_task_cache(mut self, cache: TaskCache) -> Self {
        self.task_cache = Some(cache.clone());
        self.executor = TaskExecutor::new(
            self.packages_dir.clone(),
            self.graph.clone(),
            self.command_validator.clone(),
            Some(cache),
            self.remote_cache.clone(),
        );
        self
    }

    pub fn with_max_parallel(mut self, max_parallel: Option<usize>) -> Self {
        self.max_parallel = max_parallel;
        self
    }

    pub fn with_remote_cache(mut self, remote_cache: Arc<RemoteCache>) -> Self {
        self.remote_cache = Some(remote_cache.clone());
        self.executor = TaskExecutor::new(
            self.packages_dir.clone(),
            self.graph.clone(),
            self.command_validator.clone(),
            self.task_cache.clone(),
            Some(remote_cache),
        );
        self
    }

    pub fn run_task(
        &self,
        task_name: &str,
        package_names: Option<&[String]>,
    ) -> Result<Vec<TaskResult>> {
        if let Some(names) = package_names {
            if names.is_empty() {
                return Ok(Vec::new());
            }
            if names.len() == 1 {
                if let Some(package) = self.graph.get_package(&names[0]) {
                    let result = self.executor.execute_task(package, task_name)?;
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

        if packages_to_run.is_empty() {
            return Ok(Vec::new());
        }

        let packages_set: HashSet<&str> = packages_to_run.iter().map(|p| p.name.as_str()).collect();

        let levels = self.graph.dependency_levels();
        let mut results = Vec::with_capacity(packages_to_run.len());

        for level in levels {
            let level_packages: Vec<&Package> = level
                .iter()
                .filter(|name| packages_set.contains(name.as_str()))
                .filter_map(|name| self.graph.get_package(name))
                .collect();

            if level_packages.is_empty() {
                continue;
            }

            let (tx, rx) = channel::unbounded();
            let executor = &self.executor;
            self.thread_pool.install(|| {
                level_packages
                    .into_par_iter()
                    .for_each(|package| {
                        let result = executor.execute_task(package, task_name);
                        let _ = tx.send(result);
                    });
            });
            drop(tx);

            let level_results: Result<Vec<TaskResult>> = rx
                .iter()
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| Error::TaskExecution {
                    package: "unknown".to_string(),
                    task: task_name.to_string(),
                    message: format!("Task execution failed: {}", e),
                });

            let mut level_results = level_results?;
            results.append(&mut level_results);
        }

        Ok(results)
    }

    pub async fn run_task_streaming<F>(
        &self,
        task_name: &str,
        package_names: Option<&[String]>,
        on_output: F,
    ) -> Result<Vec<TaskResult>>
    where
        F: Fn(&str, &str, bool) + Send + Sync + 'static,
    {
        let packages_to_run: Vec<Package> = if let Some(names) = package_names {
            names
                .iter()
                .filter_map(|name| self.graph.get_package(name))
                .cloned()
                .collect()
        } else {
            self.graph.all_packages().into_iter().cloned().collect()
        };

        if packages_to_run.is_empty() {
            return Ok(Vec::new());
        }

        let packages_set: HashSet<&str> = packages_to_run.iter().map(|p| p.name.as_str()).collect();

        let levels = self.graph.dependency_levels();
        let mut results = Vec::new();
        use std::sync::{Arc, Mutex};
        use tokio::sync::mpsc;

        let output_handler = Arc::new(Mutex::new(on_output));

        for level in levels {
            let level_packages: Vec<Package> = level
                .iter()
                .filter(|name| packages_set.contains(name.as_str()))
                .filter_map(|name| self.graph.get_package(name))
                .cloned()
                .collect();

            if level_packages.is_empty() {
                continue;
            }

            let (tx, mut rx) = mpsc::unbounded_channel::<(String, String, bool)>();
            let output_handler_clone = Arc::clone(&output_handler);

            let output_task = tokio::spawn(async move {
                while let Some((package_name, line, is_stderr)) = rx.recv().await {
                    if let Ok(handler) = output_handler_clone.lock() {
                        handler(&package_name, &line, is_stderr);
                    }
                }
            });

            let mut handles = Vec::new();
            let packages_dir = self.packages_dir.clone();
            for package in level_packages {
                let package_name = package.name.clone();
                let package_path = packages_dir.join(&package.path);
                let task_name = task_name.to_string();
                let tx_clone = tx.clone();

                let handle = tokio::spawn(async move {
                    let streaming_task =
                        match StreamingTask::spawn(&package, &task_name, &package_path).await {
                            Ok(task) => task,
                            Err(e) => return Err(e),
                        };

                    let stdout = Arc::new(Mutex::new(String::new()));
                    let stderr = Arc::new(Mutex::new(String::new()));
                    let stdout_clone = Arc::clone(&stdout);
                    let stderr_clone = Arc::clone(&stderr);
                    let package_name_clone = package_name.clone();

                    let success = streaming_task
                        .stream_output(move |line, is_stderr| {
                            if is_stderr {
                                if let Ok(mut stderr_guard) = stderr_clone.lock() {
                                    stderr_guard.push_str(line);
                                    stderr_guard.push('\n');
                                }
                            } else if let Ok(mut stdout_guard) = stdout_clone.lock() {
                                stdout_guard.push_str(line);
                                stdout_guard.push('\n');
                            }
                            let _ = tx_clone.send((
                                package_name_clone.clone(),
                                line.to_string(),
                                is_stderr,
                            ));
                        })
                        .await?;

                    let stdout_result = Arc::try_unwrap(stdout)
                        .map_err(|_| Error::MutexLock("Failed to unwrap stdout Arc".to_string()))?
                        .into_inner()
                        .map_err(|e| {
                            Error::MutexLock(format!("Failed to unwrap stdout Mutex: {}", e))
                        })?;

                    let stderr_result = Arc::try_unwrap(stderr)
                        .map_err(|_| Error::MutexLock("Failed to unwrap stderr Arc".to_string()))?
                        .into_inner()
                        .map_err(|e| {
                            Error::MutexLock(format!("Failed to unwrap stderr Mutex: {}", e))
                        })?;

                    Ok(TaskResult {
                        package_name,
                        task_name,
                        success,
                        stdout: stdout_result,
                        stderr: stderr_result,
                    })
                });
                handles.push(handle);
            }

            drop(tx);

            for handle in handles {
                match handle.await {
                    Ok(Ok(result)) => results.push(result),
                    Ok(Err(e)) => return Err(e),
                    Err(e) => {
                        return Err(Error::TaskExecution {
                            package: "unknown".to_string(),
                            task: task_name.to_string(),
                            message: format!("Task execution failed: {}", e),
                        });
                    }
                }
            }

            output_task.abort();
        }

        Ok(results)
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
