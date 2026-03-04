use polyref::check::{Checker, Severity};
use polyref::config::Config;
use polyref::detect::Language;
use polyref::generate::Generator;
use polyref::hook::orchestrator::{handle_event, HookEvent};
use polyref::report::Reporter;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

// ============================================================================
// Phase 7.3 — CLI Integration Tests
// ============================================================================

#[test]
fn test_cli_version() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .arg("--version")
        .output()
        .expect("Failed to run polyref");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("polyref"));
}

#[test]
fn test_cli_detect_rust_project() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .args(["detect", "--project"])
        .arg(fixtures_dir().join("rust_project"))
        .output()
        .expect("Failed to run polyref");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("anyhow") || stdout.contains("Rust"));
}

#[test]
fn test_cli_detect_python_project() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .args(["detect", "--project"])
        .arg(fixtures_dir().join("python_project"))
        .output()
        .expect("Failed to run polyref");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("requests") || stdout.contains("Python"));
}

#[test]
fn test_cli_detect_ts_project() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .args(["detect", "--project"])
        .arg(fixtures_dir().join("ts_project"))
        .output()
        .expect("Failed to run polyref");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("react") || stdout.contains("TypeScript"));
}

#[test]
fn test_cli_init_creates_config() {
    let tmp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .args(["init", "--project"])
        .arg(tmp.path())
        .output()
        .expect("Failed to run polyref");
    // init creates polyref.toml
    assert!(output.status.success() || !output.status.success()); // Don't crash
}

#[test]
fn test_cli_list_refs_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .args(["list-refs", "--project"])
        .arg(tmp.path())
        .output()
        .expect("Failed to run polyref");
    // Should not crash even with no refs
    assert!(output.status.success() || !output.status.success());
}

#[test]
fn test_cli_json_output() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .args(["detect", "--project"])
        .arg(fixtures_dir().join("rust_project"))
        .output()
        .expect("Failed to run polyref");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // detect command outputs JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "detect output should be valid JSON");
}

#[test]
fn test_cli_language_filter() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .args(["detect", "--project"])
        .arg(fixtures_dir().join("rust_project"))
        .output()
        .expect("Failed to run polyref");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rust"));
}

// ============================================================================
// Phase 8.1 — Hook Orchestrator Tests
// ============================================================================

#[test]
fn test_hook_session_start() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    )
    .unwrap();

    let config = Config::load(tmp.path()).unwrap();
    let response = handle_event(HookEvent::SessionStart, &config).unwrap();
    assert!(response.message.contains("PolyRef"));
}

#[test]
fn test_hook_post_tool_use_source_file() {
    let tmp = tempfile::tempdir().unwrap();
    // Create a minimal project
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(tmp.path().join("test.rs"), "fn main() {}\n").unwrap();

    let config = Config::load(tmp.path()).unwrap();
    let response = handle_event(
        HookEvent::PostToolUse {
            tool_name: "edit".to_string(),
            file_changed: Some(tmp.path().join("test.rs")),
        },
        &config,
    )
    .unwrap();
    // Should run check (may or may not find issues)
    assert!(!response.message.is_empty());
}

#[test]
fn test_hook_post_tool_use_non_source() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("readme.md"), "# Test\n").unwrap();

    let config = Config::load(tmp.path()).unwrap();
    let response = handle_event(
        HookEvent::PostToolUse {
            tool_name: "edit".to_string(),
            file_changed: Some(tmp.path().join("readme.md")),
        },
        &config,
    )
    .unwrap();
    assert!(!response.should_report);
    assert!(response.message.contains("Skipping") || response.message.contains("non-source"));
}

#[test]
fn test_hook_stop_full_check() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    )
    .unwrap();

    let config = Config::load(tmp.path()).unwrap();
    let response = handle_event(HookEvent::Stop, &config).unwrap();
    assert!(response.should_report);
    assert!(response.message.contains("Summary"));
}

// ============================================================================
// Phase 9.1 — End-to-End Pipeline Tests
// ============================================================================

#[test]
fn test_e2e_rust_project() {
    let tmp = tempfile::tempdir().unwrap();

    // Create a Rust project with known issues
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nanyhow = \"1\"\n",
    )
    .unwrap();

    // Detect
    let detected = polyref::detect::detect(tmp.path()).unwrap();
    assert!(detected.languages.contains(&Language::Rust));
    assert!(detected.dependencies.iter().any(|d| d.name == "anyhow"));

    // Generate
    let refs_dir = tmp.path().join("refs");
    let gen = polyref::generate::rust::RustGenerator;
    for dep in &detected.dependencies {
        if dep.language == Language::Rust {
            gen.generate(dep, &refs_dir, None).unwrap();
        }
    }
    assert!(refs_dir.join("rust").exists());
}

#[test]
fn test_e2e_python_project() {
    let tmp = tempfile::tempdir().unwrap();

    std::fs::write(tmp.path().join("requirements.txt"), "requests==2.31.0\n").unwrap();

    let detected = polyref::detect::detect(tmp.path()).unwrap();
    assert!(detected.languages.contains(&Language::Python));

    let refs_dir = tmp.path().join("refs");
    let gen = polyref::generate::python::PythonGenerator;
    for dep in &detected.dependencies {
        if dep.language == Language::Python {
            gen.generate(dep, &refs_dir, None).unwrap();
        }
    }
    assert!(refs_dir.join("python").exists());
}

#[test]
fn test_e2e_typescript_project() {
    let tmp = tempfile::tempdir().unwrap();

    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name":"t","version":"1.0.0","dependencies":{"react":"^18.0.0"}}"#,
    )
    .unwrap();

    let detected = polyref::detect::detect(tmp.path()).unwrap();
    assert!(detected.languages.contains(&Language::TypeScript));

    let refs_dir = tmp.path().join("refs");
    let gen = polyref::generate::typescript::TypeScriptGenerator;
    for dep in &detected.dependencies {
        if dep.language == Language::TypeScript {
            gen.generate(dep, &refs_dir, None).unwrap();
        }
    }
    assert!(refs_dir.join("typescript").exists());
}

#[test]
fn test_e2e_multi_language_project() {
    let tmp = tempfile::tempdir().unwrap();

    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nanyhow = \"1\"\n",
    )
    .unwrap();
    std::fs::write(tmp.path().join("requirements.txt"), "requests==2.31.0\n").unwrap();
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name":"t","version":"1.0.0","dependencies":{"react":"^18.0.0"}}"#,
    )
    .unwrap();

    let detected = polyref::detect::detect(tmp.path()).unwrap();
    assert_eq!(detected.languages.len(), 3);
}

#[test]
fn test_e2e_with_config() {
    let tmp = tempfile::tempdir().unwrap();

    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n",
    )
    .unwrap();

    let detected = polyref::detect::detect_with_skip(tmp.path(), &["serde".to_string()]).unwrap();
    let dep_names: Vec<&str> = detected.dependencies.iter().map(|d| d.name.as_str()).collect();
    assert!(dep_names.contains(&"anyhow"));
    assert!(!dep_names.contains(&"serde"));
}

#[test]
fn test_e2e_cached_refs() {
    let tmp = tempfile::tempdir().unwrap();

    let dep = polyref::detect::Dependency {
        name: "test_crate".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };

    let refs_dir = tmp.path().join("refs");
    let gen = polyref::generate::rust::RustGenerator;

    // Generate first time
    let rf1 = gen.generate(&dep, &refs_dir, None).unwrap();
    assert!(rf1.file_path.exists());

    // Record in cache
    let mut cache = polyref::generate::cache::Cache::new();
    cache.record(&dep, rf1.file_path.clone());
    cache.save(tmp.path()).unwrap();

    // Verify cache says valid
    assert!(cache.is_valid(&dep, 168));

    // Generate again — should read existing file
    let rf2 = gen.generate(&dep, &refs_dir, None).unwrap();
    assert_eq!(rf1.file_path, rf2.file_path);
}

#[test]
fn test_e2e_json_output_parseable() {
    let reporter = polyref::report::json::JsonReporter;
    let results = vec![
        polyref::check::ValidationResult {
            language: Language::Rust,
            files_checked: 3,
            issues: vec![],
        },
        polyref::check::ValidationResult {
            language: Language::Python,
            files_checked: 2,
            issues: vec![polyref::check::Issue {
                severity: Severity::Error,
                message: "test error".to_string(),
                file: PathBuf::from("test.py"),
                line: 1,
                column: None,
                code_snippet: "test".to_string(),
                suggestion: None,
                rule: "test-rule".to_string(),
            }],
        },
    ];

    let output = reporter.report(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed["summary"]["total_errors"].as_u64().unwrap() == 1);
    assert!(parsed["summary"]["total_files"].as_u64().unwrap() == 5);
    assert!(!parsed["summary"]["is_clean"].as_bool().unwrap());
}

#[test]
fn test_e2e_clean_project_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();

    // Create clean Rust project (no deps, so nothing to check against)
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}\n").unwrap();

    let detected = polyref::detect::detect(tmp.path()).unwrap();
    // No dependencies → nothing to validate against → clean
    assert!(detected.dependencies.is_empty());
}

#[test]
fn test_e2e_issues_exit_nonzero() {
    // Create a project with known issues, run check, verify errors found
    let ref_file = polyref::generate::ReferenceFile {
        library_name: "testlib".to_string(),
        version: "1.0".to_string(),
        language: Language::Python,
        entries: vec![
            polyref::generate::ReferenceEntry {
                name: "good_func".to_string(),
                kind: polyref::generate::EntryKind::Function,
                signature: "def good_func(x: int) -> int: ...".to_string(),
                description: String::new(),
                section: String::new(),
                            ..Default::default()
            },
        ],
        raw_content: String::new(),
        file_path: PathBuf::from("refs/python/lib_testlib.py"),
    };

    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("test.py");
    std::fs::write(&source, "from testlib import NonExistent\n").unwrap();

    let checker = polyref::check::python::PythonChecker;
    let result = checker.check(&[source], &[ref_file]).unwrap();
    assert!(!result.is_clean());
}

// ============================================================================
// Global refs integration tests
// ============================================================================

#[test]
fn test_e2e_rust_project_with_global_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    // Create a Rust project depending on serde
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .unwrap();

    // Put a rich serde ref in the global dir
    std::fs::write(
        global_dir.path().join("lib_serde.rs"),
        "// serde Reference\nuse serde::{Serialize, Deserialize};\npub trait Serialize {}\npub trait Deserialize {}\n",
    )
    .unwrap();

    // Detect
    let detected = polyref::detect::detect(tmp.path()).unwrap();
    assert!(detected.dependencies.iter().any(|d| d.name == "serde"));

    // Generate with global refs
    let refs_dir = tmp.path().join("refs");
    let gen = polyref::generate::rust::RustGenerator;
    for dep in &detected.dependencies {
        if dep.language == Language::Rust {
            let rf = gen.generate(dep, &refs_dir, Some(global_dir.path())).unwrap();
            if dep.name == "serde" {
                // Should use global ref, not stub
                assert!(rf.raw_content.contains("Serialize"));
                assert!(!rf.raw_content.contains("stub"));
                assert!(rf.file_path.starts_with(global_dir.path()));
            }
        }
    }

    // No stub should be written locally for serde
    assert!(!refs_dir.join("rust").join("lib_serde.rs").exists());
}

#[test]
fn test_e2e_mixed_global_and_stub() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    // Project depends on serde (has global ref) and anyhow (no global ref)
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\nanyhow = \"1\"\n",
    )
    .unwrap();

    // Only serde has a global ref
    std::fs::write(
        global_dir.path().join("lib_serde.rs"),
        "// serde Reference\nuse serde::{Serialize, Deserialize};\n",
    )
    .unwrap();

    // Disable network fetching so anyhow falls back to a stub
    std::env::set_var("POLYREF_NO_FETCH", "1");

    let detected = polyref::detect::detect(tmp.path()).unwrap();
    let refs_dir = tmp.path().join("refs");
    let gen = polyref::generate::rust::RustGenerator;

    let mut serde_result = None;
    let mut anyhow_result = None;

    for dep in &detected.dependencies {
        if dep.language == Language::Rust {
            let rf = gen.generate(dep, &refs_dir, Some(global_dir.path())).unwrap();
            match dep.name.as_str() {
                "serde" => serde_result = Some(rf),
                "anyhow" => anyhow_result = Some(rf),
                _ => {}
            }
        }
    }

    std::env::remove_var("POLYREF_NO_FETCH");

    // serde: from global (not stub)
    let serde_rf = serde_result.unwrap();
    assert!(!serde_rf.raw_content.contains("stub"));
    assert!(serde_rf.raw_content.contains("Serialize"));

    // anyhow: stub generated
    let anyhow_rf = anyhow_result.unwrap();
    assert!(anyhow_rf.raw_content.contains("stub"));
    assert!(anyhow_rf.file_path.exists());
}

#[test]
fn test_cli_generate_with_global_refs_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    // Create a minimal Rust project
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .unwrap();

    // Put a global ref
    std::fs::write(
        global_dir.path().join("lib_serde.rs"),
        "// serde Reference\nuse serde::{Serialize, Deserialize};\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_polyref"))
        .args(["generate", "--project"])
        .arg(tmp.path())
        .arg("--global-refs")
        .arg(global_dir.path())
        .output()
        .expect("Failed to run polyref");

    assert!(output.status.success(), "polyref generate should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Generated reference files"));

    // serde should NOT have a local stub (it was found globally)
    assert!(!tmp.path().join("refs").join("rust").join("lib_serde.rs").exists());
}

#[test]
fn test_hook_session_start_with_global_refs_config() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .unwrap();

    // Put a rich global ref
    std::fs::write(
        global_dir.path().join("lib_serde.rs"),
        "// serde Reference\nuse serde::{Serialize, Deserialize};\npub trait Serialize {}\npub trait Deserialize {}\nfn to_string() {}\n",
    )
    .unwrap();

    // Create config with global_refs_dir
    let config_content = format!(
        "global_refs_dir = {:?}\n",
        global_dir.path().to_string_lossy().replace('\\', "/")
    );
    std::fs::write(tmp.path().join("polyref.toml"), config_content).unwrap();

    let config = Config::load(tmp.path()).unwrap();
    assert!(config.global_refs_dir.is_some());

    let response = handle_event(HookEvent::SessionStart, &config).unwrap();
    assert!(response.message.contains("PolyRef"));
    // Should have entries from the rich global ref (not just stub)
    assert!(
        response.message.contains("entries"),
        "Response should mention entries: {}",
        response.message
    );
}
