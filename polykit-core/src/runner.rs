//! Task execution engine.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use rayon::prelude::*;

use std::sync::Arc;

use crate::command_validator::CommandValidator;
use crate::error::{Error, Result};
use crate::graph::DependencyGraph;
use crate::package::Package;
use crate::remote_cache::{Artifact, ArtifactVerifier, RemoteCache};
use crate::simd_utils;
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
}

impl TaskRunner {
    pub fn new(packages_dir: impl Into<PathBuf>, graph: DependencyGraph) -> Self {
        Self {
            packages_dir: packages_dir.into(),
            graph,
            max_parallel: None,
            command_validator: CommandValidator::new(),
            task_cache: None,
            remote_cache: None,
        }
    }

    pub fn with_command_validator(mut self, validator: CommandValidator) -> Self {
        self.command_validator = validator;
        self
    }

    pub fn with_task_cache(mut self, cache: TaskCache) -> Self {
        self.task_cache = Some(cache);
        self
    }

    pub fn with_max_parallel(mut self, max_parallel: Option<usize>) -> Self {
        self.max_parallel = max_parallel;
        self
    }

    pub fn with_remote_cache(mut self, remote_cache: Arc<RemoteCache>) -> Self {
        self.remote_cache = Some(remote_cache);
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

        if packages_to_run.is_empty() {
            return Ok(Vec::new());
        }

        let packages_set: HashSet<&str> = packages_to_run.iter().map(|p| p.name.as_str()).collect();

        let levels = self.graph.dependency_levels();
        let mut results = Vec::with_capacity(packages_to_run.len());

        let thread_count = self.max_parallel.unwrap_or_else(rayon::current_num_threads);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

        for level in levels {
            let level_packages: Vec<&Package> = level
                .iter()
                .filter(|name| packages_set.contains(name.as_str()))
                .filter_map(|name| self.graph.get_package(name))
                .collect();

            if level_packages.is_empty() {
                continue;
            }

            let level_results: Result<Vec<TaskResult>> = pool.install(|| {
                level_packages
                    .into_par_iter()
                    .map(|package| self.execute_task(package, task_name))
                    .collect()
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
        let mut results = Vec::with_capacity(task_order.len());

        for task in &task_order {
            let result = self.execute_task_internal(package, task)?;
            let success = result.success;
            results.push(result);
            if !success && task == task_name {
                return Ok(results);
            }
        }

        Ok(results)
    }

    fn execute_task_internal(&self, package: &Package, task_name: &str) -> Result<TaskResult> {
        let task = package.get_task(task_name).ok_or_else(|| {
            let available_tasks: Vec<&str> =
                package.tasks.iter().map(|t| t.name.as_str()).collect();
            Error::TaskExecution {
                package: package.name.clone(),
                task: task_name.to_string(),
                message: format!(
                    "Task '{}' not found. Available tasks: {}",
                    task_name,
                    available_tasks.join(", ")
                ),
            }
        })?;

        let package_path = self.packages_dir.join(&package.path);

        if let Some(ref remote_cache) = self.remote_cache {
            match self.check_remote_cache(
                remote_cache,
                package,
                task_name,
                &task.command,
                &package_path,
            ) {
                Ok(Some(cached_result)) => return Ok(cached_result),
                Ok(None) => {}
                Err(_) => {}
            }
        }

        if let Some(ref cache) = self.task_cache {
            if let Some(cached_result) = cache.get(&package.name, task_name, &task.command)? {
                return Ok(cached_result);
            }
        }

        self.command_validator.validate(&task.command)?;

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

        let stdout = if simd_utils::is_ascii_fast(&output.stdout) {
            unsafe { String::from_utf8_unchecked(output.stdout) }
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        let stderr = if simd_utils::is_ascii_fast(&output.stderr) {
            unsafe { String::from_utf8_unchecked(output.stderr) }
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };

        let result = TaskResult {
            package_name: package.name.clone(),
            task_name: task_name.to_string(),
            success: output.status.success(),
            stdout,
            stderr,
        };

        // Store in local cache
        if let Some(ref cache) = self.task_cache {
            let _ = cache.put(&package.name, task_name, &task.command, &result);
        }

        if result.success {
            if let Some(ref remote_cache) = self.remote_cache {
                let _ = self.upload_to_remote_cache(
                    remote_cache,
                    package,
                    task_name,
                    &task.command,
                    &package_path,
                    &result,
                );
            }
        }

        Ok(result)
    }

    /// Checks remote cache for a task result.
    ///
    /// Returns `Ok(Some(result))` if found, `Ok(None)` if not found, or `Err` on error.
    fn check_remote_cache(
        &self,
        remote_cache: &RemoteCache,
        package: &Package,
        task_name: &str,
        command: &str,
        package_path: &std::path::Path,
    ) -> Result<Option<TaskResult>> {
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => {
                // Create a new runtime if we're not in an async context
                tokio::runtime::Runtime::new()
                    .map_err(|e| Error::Adapter {
                        package: "remote-cache".to_string(),
                        message: format!("Failed to create tokio runtime: {}", e),
                    })?
                    .handle()
                    .clone()
            }
        };

        // Build cache key
        let cache_key = rt.block_on(remote_cache.build_cache_key(
            package,
            task_name,
            command,
            &self.graph,
            package_path,
        ))?;

        // Fetch artifact
        let artifact_opt = rt.block_on(remote_cache.fetch_artifact(&cache_key))?;

        if let Some(artifact) = artifact_opt {
            if ArtifactVerifier::verify(&artifact, None).is_err() {
                return Err(Error::Adapter {
                    package: "remote-cache".to_string(),
                    message: "Artifact integrity verification failed".to_string(),
                });
            }

            // Extract outputs
            artifact.extract_outputs(package_path)?;

            // Return cached result
            Ok(Some(TaskResult {
                package_name: package.name.clone(),
                task_name: task_name.to_string(),
                success: true,
                stdout: String::new(), // Outputs are in files, not stdout
                stderr: String::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Uploads task result to remote cache.
    fn upload_to_remote_cache(
        &self,
        remote_cache: &RemoteCache,
        package: &Package,
        task_name: &str,
        command: &str,
        package_path: &std::path::Path,
        result: &TaskResult,
    ) -> Result<()> {
        use std::collections::BTreeMap;

        let rt = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => {
                tokio::runtime::Runtime::new()
                    .map_err(|e| Error::Adapter {
                        package: "remote-cache".to_string(),
                        message: format!("Failed to create tokio runtime: {}", e),
                    })?
                    .handle()
                    .clone()
            }
        };

        // Build cache key
        let cache_key = rt.block_on(remote_cache.build_cache_key(
            package,
            task_name,
            command,
            &self.graph,
            package_path,
        ))?;

        // Collect output files (simplified - in practice, we'd track what files were created)
        // For now, we'll create a minimal artifact with stdout/stderr
        let mut output_files = BTreeMap::new();
        output_files.insert(
            PathBuf::from("stdout.txt"),
            result.stdout.as_bytes().to_vec(),
        );
        output_files.insert(
            PathBuf::from("stderr.txt"),
            result.stderr.as_bytes().to_vec(),
        );

        // Create artifact
        let artifact = Artifact::new(
            package.name.clone(),
            task_name.to_string(),
            command.to_string(),
            cache_key.as_string(),
            output_files,
        )?;

        let _ = rt.block_on(remote_cache.upload_artifact(&cache_key, &artifact));

        Ok(())
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
