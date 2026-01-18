use polykit_core::runner::TaskResult;
use polykit_core::task_cache::TaskCache;
use tempfile::TempDir;

#[test]
fn test_task_cache_put_and_get() {
    let temp_dir = TempDir::new().unwrap();
    let cache = TaskCache::new(temp_dir.path());

    let result = TaskResult {
        package_name: "test-pkg".to_string(),
        task_name: "test".to_string(),
        success: true,
        stdout: "output".to_string(),
        stderr: "".to_string(),
    };

    cache.put("test-pkg", "test", "echo test", &result).unwrap();

    let cached = cache.get("test-pkg", "test", "echo test").unwrap();
    assert!(cached.is_some());
    let cached_result = cached.unwrap();
    assert_eq!(cached_result.package_name, "test-pkg");
    assert_eq!(cached_result.task_name, "test");
    assert_eq!(cached_result.stdout, "output");
}

#[test]
fn test_task_cache_miss() {
    let temp_dir = TempDir::new().unwrap();
    let cache = TaskCache::new(temp_dir.path());

    let cached = cache.get("test-pkg", "test", "echo test").unwrap();
    assert!(cached.is_none());
}

#[test]
fn test_task_cache_failed_task_not_cached() {
    let temp_dir = TempDir::new().unwrap();
    let cache = TaskCache::new(temp_dir.path());

    let result = TaskResult {
        package_name: "test-pkg".to_string(),
        task_name: "test".to_string(),
        success: false,
        stdout: "".to_string(),
        stderr: "error".to_string(),
    };

    cache.put("test-pkg", "test", "false", &result).unwrap();

    let cached = cache.get("test-pkg", "test", "false").unwrap();
    assert!(cached.is_none());
}
