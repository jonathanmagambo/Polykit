//! Internal task execution logic.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::command_validator::CommandValidator;
use crate::error::{Error, Result};
use crate::graph::DependencyGraph;
use crate::package::Package;
use crate::remote_cache::{Artifact, ArtifactVerifier, RemoteCache};
use crate::runner::TaskResult;
use crate::simd_utils;
use crate::task_cache::TaskCache;

pub struct TaskExecutor {
    packages_dir: PathBuf,
    graph: Arc<DependencyGraph>,
    command_validator: CommandValidator,
    task_cache: Option<TaskCache>,
    remote_cache: Option<Arc<RemoteCache>>,
}

impl TaskExecutor {
    pub fn new(
        packages_dir: PathBuf,
        graph: DependencyGraph,
        command_validator: CommandValidator,
        task_cache: Option<TaskCache>,
        remote_cache: Option<Arc<RemoteCache>>,
    ) -> Self {
        Self {
            packages_dir,
            graph: Arc::new(graph),
            command_validator,
            task_cache,
            remote_cache,
        }
    }

    pub fn build_task_dependency_order(
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

    pub fn execute_task_with_deps(
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

    pub fn execute_task_internal(&self, package: &Package, task_name: &str) -> Result<TaskResult> {
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
                let remote_cache = Arc::clone(remote_cache);
                let package = package.clone();
                let task_name = task_name.to_string();
                let command = task.command.clone();
                let package_path = package_path.to_path_buf();
                let result = result.clone();

                tokio::spawn(async move {
                    let rt = tokio::runtime::Handle::try_current()
                        .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone());
                    rt.block_on(async {
                        use std::collections::BTreeMap;
                        let mut output_files = BTreeMap::new();
                        output_files.insert(
                            PathBuf::from("stdout.txt"),
                            result.stdout.as_bytes().to_vec(),
                        );
                        output_files.insert(
                            PathBuf::from("stderr.txt"),
                            result.stderr.as_bytes().to_vec(),
                        );
                        let temp_graph = DependencyGraph::new(vec![package.clone()]).ok();
                        if let Some(ref graph) = temp_graph {
                            if let Ok(cache_key) = remote_cache
                                .build_cache_key(&package, &task_name, &command, graph, &package_path)
                                .await
                            {
                                if let Ok(artifact) = Artifact::new(
                                    package.name.clone(),
                                    task_name.clone(),
                                    command.clone(),
                                    cache_key.as_string(),
                                    output_files,
                                ) {
                                    let _ = remote_cache.upload_artifact(&cache_key, &artifact).await;
                                }
                            }
                        }
                    });
                });
            }
        }

        Ok(result)
    }

    /// Checks remote cache for a task result.
    ///
    /// Returns `Ok(Some(result))` if found, `Ok(None)` if not found, or `Err` on error.
    pub fn check_remote_cache(
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
            self.graph.as_ref(),
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

    pub fn execute_task(&self, package: &Package, task_name: &str) -> Result<TaskResult> {
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
