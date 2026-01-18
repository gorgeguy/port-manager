//! Integration tests for the port-manager CLI.
//!
//! These tests verify end-to-end CLI behavior using a temporary config file.

#![allow(deprecated)] // cargo_bin works fine for standard builds

use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Creates a new command with a temporary config path.
fn pm_cmd(config_path: &str) -> assert_cmd::Command {
    let mut cmd = Command::cargo_bin("pm").unwrap();
    cmd.env("PM_CONFIG_PATH", config_path);
    assert_cmd::Command::from_std(cmd)
}

/// Creates a temporary directory and returns the path for the config file.
fn setup_temp_config() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("registry.toml");
    (temp_dir, config_path.to_string_lossy().to_string())
}

// ============================================================================
// Allocation Flow Tests
// ============================================================================

#[test]
fn test_allocate_auto_port() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Allocated webapp.web ="));
}

#[test]
fn test_allocate_explicit_port() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8080"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Allocated webapp.web = 8080"));
}

#[test]
fn test_allocate_then_query() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate a port
    pm_cmd(&config_path)
        .args(["allocate", "myapp", "api", "3000"])
        .assert()
        .success();

    // Query the allocated port
    pm_cmd(&config_path)
        .args(["query", "myapp", "api"])
        .assert()
        .success()
        .stdout(predicate::str::contains("3000"));
}

#[test]
fn test_allocate_query_all_ports() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate multiple ports
    pm_cmd(&config_path)
        .args(["allocate", "myapp", "web", "8080"])
        .assert()
        .success();

    pm_cmd(&config_path)
        .args(["allocate", "myapp", "api", "3000"])
        .assert()
        .success();

    // Query all ports for the project
    pm_cmd(&config_path)
        .args(["query", "myapp"])
        .assert()
        .success()
        .stdout(predicate::str::contains("web=8080"))
        .stdout(predicate::str::contains("api=3000"));
}

#[test]
fn test_allocate_then_free() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate a port
    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8080"])
        .assert()
        .success();

    // Free the port
    pm_cmd(&config_path)
        .args(["free", "webapp", "web"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Freed webapp.web (was 8080)"));

    // Query should fail since project is now gone (no ports left)
    pm_cmd(&config_path)
        .args(["query", "webapp", "web"])
        .assert()
        .failure();
}

#[test]
fn test_free_all_ports() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate multiple ports
    pm_cmd(&config_path)
        .args(["allocate", "myapp", "web", "8080"])
        .assert()
        .success();

    pm_cmd(&config_path)
        .args(["allocate", "myapp", "api", "3000"])
        .assert()
        .success();

    // Free all ports for the project
    pm_cmd(&config_path)
        .args(["free", "myapp"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Freed myapp."));

    // Query should fail since project is now gone
    pm_cmd(&config_path)
        .args(["query", "myapp"])
        .assert()
        .failure();
}

// ============================================================================
// Config Command Tests
// ============================================================================

#[test]
fn test_config_show_defaults() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("web"))
        .stdout(predicate::str::contains("api"))
        .stdout(predicate::str::contains("db"));
}

#[test]
fn test_config_show_path() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["config", "--path"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&config_path));
}

#[test]
fn test_config_set_range() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Set a custom range
    pm_cmd(&config_path)
        .args(["config", "--set", "custom=7000-7999"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set custom range to 7000-7999"));

    // Verify it's persisted by suggesting from that range
    pm_cmd(&config_path)
        .args(["suggest", "--type", "custom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("7"));
}

// ============================================================================
// List Command Tests
// ============================================================================

#[test]
fn test_list_empty() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path).args(["list"]).assert().success();
}

#[test]
fn test_list_with_allocations() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate some ports
    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8080"])
        .assert()
        .success();

    pm_cmd(&config_path)
        .args(["allocate", "backend", "api", "3000"])
        .assert()
        .success();

    // List should show both
    pm_cmd(&config_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("webapp"))
        .stdout(predicate::str::contains("backend"))
        .stdout(predicate::str::contains("8080"))
        .stdout(predicate::str::contains("3000"));
}

// ============================================================================
// Status Command Tests
// ============================================================================

#[test]
fn test_status_runs() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Status command should run successfully (output depends on system state)
    pm_cmd(&config_path).args(["status"]).assert().success();
}

// ============================================================================
// Suggest Command Tests
// ============================================================================

#[test]
fn test_suggest_default() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Suggest outputs just the port number (for scripting)
    pm_cmd(&config_path)
        .args(["suggest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("9")); // default range 9000-9999
}

#[test]
fn test_suggest_multiple() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["suggest", "3"])
        .assert()
        .success();
}

#[test]
fn test_suggest_by_type() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Suggest web ports (8000-8999 range)
    pm_cmd(&config_path)
        .args(["suggest", "--type", "web"])
        .assert()
        .success()
        .stdout(predicate::str::contains("8"));
}

// ============================================================================
// Error Case Tests
// ============================================================================

#[test]
fn test_allocate_duplicate_port() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate a port
    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8080"])
        .assert()
        .success();

    // Try to allocate the same port to another project
    pm_cmd(&config_path)
        .args(["allocate", "other", "web", "8080"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already allocated"));
}

#[test]
fn test_allocate_same_name_twice() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate a port
    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8080"])
        .assert()
        .success();

    // Try to allocate the same name again
    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8081"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_free_nonexistent_project() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["free", "nonexistent", "web"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_query_nonexistent_project() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["query", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_invalid_port_number() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "0"])
        .assert()
        .failure();
}

#[test]
fn test_invalid_port_too_large() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "99999"])
        .assert()
        .failure();
}

#[test]
fn test_config_invalid_range_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["config", "--set", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("format"));
}

// ============================================================================
// Alias Tests
// ============================================================================

#[test]
fn test_allocate_alias() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path)
        .args(["a", "webapp", "web", "8080"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Allocated webapp.web = 8080"));
}

#[test]
fn test_free_alias() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate first
    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8080"])
        .assert()
        .success();

    // Free using alias
    pm_cmd(&config_path)
        .args(["f", "webapp", "web"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Freed"));
}

#[test]
fn test_list_alias() {
    let (_temp_dir, config_path) = setup_temp_config();

    pm_cmd(&config_path).args(["ls"]).assert().success();
}

#[test]
fn test_query_alias() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate first
    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8080"])
        .assert()
        .success();

    pm_cmd(&config_path)
        .args(["q", "webapp"])
        .assert()
        .success()
        .stdout(predicate::str::contains("8080"));
}

// ============================================================================
// Persistence Tests
// ============================================================================

#[test]
fn test_config_persists() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate a port
    pm_cmd(&config_path)
        .args(["allocate", "webapp", "web", "8080"])
        .assert()
        .success();

    // Check the config file was created and contains the allocation
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("webapp"));
    assert!(content.contains("8080"));
}

#[test]
fn test_multiple_projects() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Allocate ports to multiple projects
    pm_cmd(&config_path)
        .args(["allocate", "frontend", "web", "8080"])
        .assert()
        .success();

    pm_cmd(&config_path)
        .args(["allocate", "backend", "api", "3000"])
        .assert()
        .success();

    pm_cmd(&config_path)
        .args(["allocate", "database", "db", "5432"])
        .assert()
        .success();

    // Query each project
    pm_cmd(&config_path)
        .args(["query", "frontend", "web"])
        .assert()
        .success()
        .stdout(predicate::str::contains("8080"));

    pm_cmd(&config_path)
        .args(["query", "backend", "api"])
        .assert()
        .success()
        .stdout(predicate::str::contains("3000"));

    pm_cmd(&config_path)
        .args(["query", "database", "db"])
        .assert()
        .success()
        .stdout(predicate::str::contains("5432"));
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[test]
fn test_concurrent_allocations_no_duplicates() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Spawn multiple processes concurrently trying to allocate ports
    let mut handles = vec![];
    for i in 0..5 {
        let path = config_path.clone();
        let handle = std::thread::spawn(move || {
            let mut cmd = Command::cargo_bin("pm").unwrap();
            cmd.env("PM_CONFIG_PATH", &path);
            cmd.args(["allocate", &format!("project{}", i), "web"]);
            cmd.output().unwrap()
        });
        handles.push(handle);
    }

    // Wait for all threads and collect results
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All should succeed
    for result in &results {
        assert!(result.status.success(), "Allocation failed: {:?}", result);
    }

    // Verify all ports are unique using list command
    pm_cmd(&config_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("project0"))
        .stdout(predicate::str::contains("project1"))
        .stdout(predicate::str::contains("project2"))
        .stdout(predicate::str::contains("project3"))
        .stdout(predicate::str::contains("project4"));

    // Read the config file and verify no duplicate ports
    let content = fs::read_to_string(&config_path).unwrap();
    // Match port allocations (single number) not range definitions (arrays)
    let ports: Vec<u16> = content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("web = ") && !trimmed.contains('[') {
                // Extract the port number (e.g., "web = 8000" -> 8000)
                trimmed.strip_prefix("web = ")?.parse().ok()
            } else {
                None
            }
        })
        .collect();

    // Should have 5 unique port assignments
    assert_eq!(ports.len(), 5, "Expected 5 port assignments");

    // Check all ports are unique
    let mut unique_ports = ports.clone();
    unique_ports.sort();
    unique_ports.dedup();
    assert_eq!(unique_ports.len(), 5, "All ports should be unique");
}
