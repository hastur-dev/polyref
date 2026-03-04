use clap::Parser;
use polyref::check::Checker;
use polyref::config::Config;
use polyref::coverage;
use polyref::detect::Language;
use polyref::enforce::{
    build_enforce_result, format_enforce_result, EnforceConfig, EnforceVerdict,
    OutputFormat,
};
use polyref::generate::Generator;
use polyref::report::Reporter;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "polyref", version, about = "Multi-language library reference generator and code validator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Output format: terminal, json, both
    #[arg(short, long, global = true, default_value = "terminal")]
    output: String,

    /// Filter to specific language
    #[arg(short, long, global = true)]
    language: Option<String>,

    /// Skip specific library (can be repeated)
    #[arg(long, global = true)]
    skip: Vec<String>,

    /// Disable reference file caching
    #[arg(long, global = true)]
    no_cache: bool,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Global directory of existing reference files (flat layout)
    #[arg(long, global = true, value_name = "DIR")]
    global_refs: Option<String>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Detect languages and dependencies in a project
    Detect {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        project: String,
    },
    /// Generate reference files for detected dependencies
    Generate {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        project: String,
    },
    /// Validate source code against reference files
    Check {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        project: String,
    },
    /// Full pipeline: detect -> generate -> check -> report
    Run {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        project: String,
    },
    /// Create a polyref.toml configuration file
    Init {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        project: String,
    },
    /// List all generated reference files
    ListRefs {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        project: String,
    },
    /// Enforce API correctness on source code
    Enforce {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        project: String,
        /// Block on issues (exit 1 if issues found)
        #[arg(long)]
        enforce: bool,
        /// Read source from stdin instead of project files
        #[arg(long)]
        from_stdin: bool,
        /// Treat uncovered packages as blocking
        #[arg(long)]
        strict: bool,
        /// Minimum coverage percentage required (1-100)
        #[arg(long, value_name = "PCT")]
        require_coverage: Option<u8>,
        /// Output format: human or json
        #[arg(long, default_value = "human", value_name = "FORMAT")]
        output_format: String,
        /// Language hint: rust, python, typescript, or auto
        #[arg(long, default_value = "auto")]
        lang: String,
        /// Reference files directory
        #[arg(long, value_name = "DIR")]
        refs: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let global_refs = cli.global_refs.as_deref();

    match cli.command {
        Some(Commands::Detect { project }) => cmd_detect(&project, &cli.skip),
        Some(Commands::Generate { project }) => cmd_generate(&project, &cli.skip, cli.verbose, global_refs),
        Some(Commands::Check { project }) => cmd_check(&project, &cli.output, &cli.skip, cli.verbose, global_refs),
        Some(Commands::Run { project }) => cmd_run(&project, &cli.output, &cli.skip, cli.verbose, global_refs),
        Some(Commands::Init { project }) => cmd_init(&project),
        Some(Commands::ListRefs { project }) => cmd_list_refs(&project),
        Some(Commands::Enforce {
            project,
            enforce,
            from_stdin,
            strict,
            require_coverage,
            output_format,
            lang,
            refs,
        }) => cmd_enforce(
            &project,
            enforce,
            from_stdin,
            strict,
            require_coverage,
            &output_format,
            &lang,
            refs.as_deref(),
            global_refs,
        ),
        None => {
            println!("No command specified. Use --help for usage.");
            Ok(())
        }
    }
}

fn cmd_detect(project: &str, skip: &[String]) -> anyhow::Result<()> {
    let project_root = Path::new(project);
    let detected = polyref::detect::detect_with_skip(project_root, skip)?;
    println!("{}", serde_json::to_string_pretty(&detected)?);
    Ok(())
}

fn resolve_global_refs(cli_override: Option<&str>, config: &Config) -> Option<std::path::PathBuf> {
    if let Some(cli_dir) = cli_override {
        Some(std::path::PathBuf::from(cli_dir))
    } else {
        config.resolved_global_refs_dir()
    }
}

fn cmd_generate(project: &str, skip: &[String], verbose: bool, global_refs_override: Option<&str>) -> anyhow::Result<()> {
    let project_root = Path::new(project);
    let config = Config::load(project_root)?;
    let detected = polyref::detect::detect_with_skip(project_root, skip)?;
    let refs_dir = config.resolved_refs_dir();
    let global_refs = resolve_global_refs(global_refs_override, &config);
    let global_refs_path = global_refs.as_deref();

    let mut count = 0;
    for dep in &detected.dependencies {
        if verbose {
            println!("Generating reference for {} ({})", dep.name, dep.language);
        }
        match dep.language {
            Language::Rust => {
                let gen = polyref::generate::rust::RustGenerator;
                gen.generate(dep, &refs_dir, global_refs_path)?;
            }
            Language::Python => {
                let gen = polyref::generate::python::PythonGenerator;
                gen.generate(dep, &refs_dir, global_refs_path)?;
            }
            Language::TypeScript => {
                let gen = polyref::generate::typescript::TypeScriptGenerator;
                gen.generate(dep, &refs_dir, global_refs_path)?;
            }
        }
        count += 1;
    }

    println!("Generated reference files for {} dependencies", count);
    Ok(())
}

fn cmd_check(project: &str, output_format: &str, skip: &[String], verbose: bool, global_refs_override: Option<&str>) -> anyhow::Result<()> {
    let project_root = Path::new(project);
    let config = Config::load(project_root)?;
    let detected = polyref::detect::detect_with_skip(project_root, skip)?;
    let refs_dir = config.resolved_refs_dir();
    let global_refs = resolve_global_refs(global_refs_override, &config);
    let global_refs_path = global_refs.as_deref();

    // Load/generate references
    let mut ref_files = Vec::new();
    for dep in &detected.dependencies {
        let rf = match dep.language {
            Language::Rust => {
                let gen = polyref::generate::rust::RustGenerator;
                gen.generate(dep, &refs_dir, global_refs_path)?
            }
            Language::Python => {
                let gen = polyref::generate::python::PythonGenerator;
                gen.generate(dep, &refs_dir, global_refs_path)?
            }
            Language::TypeScript => {
                let gen = polyref::generate::typescript::TypeScriptGenerator;
                gen.generate(dep, &refs_dir, global_refs_path)?
            }
        };
        ref_files.push(rf);
    }

    // Find source files
    let source_files = find_source_files(project_root, &detected.languages);
    if verbose {
        println!("Found {} source files to check", source_files.len());
    }

    // Run checkers
    let mut results = Vec::new();

    let rs_files: Vec<_> = source_files.iter().filter(|f| f.extension().and_then(|e| e.to_str()) == Some("rs")).cloned().collect();
    if !rs_files.is_empty() {
        let checker = polyref::check::rust::RustChecker;
        results.push(checker.check(&rs_files, &ref_files)?);
    }

    let py_files: Vec<_> = source_files.iter().filter(|f| f.extension().and_then(|e| e.to_str()) == Some("py")).cloned().collect();
    if !py_files.is_empty() {
        let checker = polyref::check::python::PythonChecker;
        results.push(checker.check(&py_files, &ref_files)?);
    }

    let ts_files: Vec<_> = source_files.iter().filter(|f| {
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
        ext == "ts" || ext == "tsx"
    }).cloned().collect();
    if !ts_files.is_empty() {
        let checker = polyref::check::typescript::TypeScriptChecker;
        results.push(checker.check(&ts_files, &ref_files)?);
    }

    // Report
    print_results(&results, output_format)?;

    // Exit with non-zero if errors found
    let total_errors: usize = results.iter().map(|r| r.error_count()).sum();
    if total_errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_run(project: &str, output_format: &str, skip: &[String], verbose: bool, global_refs_override: Option<&str>) -> anyhow::Result<()> {
    if verbose {
        println!("Running full pipeline: detect -> generate -> check -> report");
    }
    cmd_generate(project, skip, verbose, global_refs_override)?;
    cmd_check(project, output_format, skip, verbose, global_refs_override)
}

fn cmd_init(project: &str) -> anyhow::Result<()> {
    let project_root = Path::new(project);
    let config_path = project_root.join("polyref.toml");

    if config_path.exists() {
        println!("polyref.toml already exists at {}", config_path.display());
        return Ok(());
    }

    let default_config = r#"# PolyRef Configuration
# See https://github.com/polyref for documentation

# Where to store generated reference files
refs_dir = "refs"

# Libraries to skip (don't generate references for these)
skip_libraries = []

# Output format: Terminal, Json, or Both
output_format = "Terminal"

# Whether to use cached reference files
use_cache = true

# Maximum age of cached files in hours (168 = 1 week)
cache_max_age_hours = 168

# Optional: global directory of existing reference files (flat layout)
# If set, polyref checks this directory before generating stubs
# global_refs_dir = "C:/Users/you/Documents/coding/references"
"#;

    std::fs::write(&config_path, default_config)?;
    println!("Created polyref.toml at {}", config_path.display());
    Ok(())
}

fn cmd_list_refs(project: &str) -> anyhow::Result<()> {
    let project_root = Path::new(project);
    let config = Config::load(project_root)?;
    let refs_dir = config.resolved_refs_dir();

    if !refs_dir.exists() {
        println!("No reference files directory found at {}", refs_dir.display());
        return Ok(());
    }

    let mut count = 0;
    for lang_dir in &["rust", "python", "typescript"] {
        let dir = refs_dir.join(lang_dir);
        if !dir.exists() {
            continue;
        }

        println!("\n{}:", lang_dir.to_uppercase());
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    println!("  {} ({} bytes)", name, size);
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        println!("No reference files found.");
    } else {
        println!("\nTotal: {} reference files", count);
    }

    Ok(())
}

fn print_results(results: &[polyref::check::ValidationResult], output_format: &str) -> anyhow::Result<()> {
    match output_format {
        "json" => {
            let reporter = polyref::report::json::JsonReporter;
            println!("{}", reporter.report(results)?);
        }
        "both" => {
            let terminal = polyref::report::terminal::TerminalReporter;
            print!("{}", terminal.report(results)?);
            let json = polyref::report::json::JsonReporter;
            println!("{}", json.report(results)?);
        }
        _ => {
            let reporter = polyref::report::terminal::TerminalReporter;
            print!("{}", reporter.report(results)?);
        }
    }
    Ok(())
}

/// Read source code from stdin or a file path.
/// Returns (content, display_path) on success.
fn read_source_input(
    path: Option<&str>,
    from_stdin: bool,
) -> anyhow::Result<(String, String)> {
    if from_stdin {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        if buf.trim().is_empty() {
            anyhow::bail!("stdin was empty");
        }
        return Ok((buf, "<stdin>".to_string()));
    }

    let file_path = path.ok_or_else(|| anyhow::anyhow!("no source file specified"))?;
    let content = std::fs::read_to_string(file_path)?;
    if content.trim().is_empty() {
        anyhow::bail!("source file is empty: {}", file_path);
    }
    Ok((content, file_path.to_string()))
}

/// Map CLI args to an EnforceConfig.
fn build_enforce_config_from_args(
    enforce: bool,
    strict: bool,
    require_coverage: Option<u8>,
    from_stdin: bool,
    output_format: &str,
) -> EnforceConfig {
    EnforceConfig {
        hard_block: enforce,
        strict_unknown_packages: strict,
        require_coverage,
        from_stdin,
        output_format: match output_format {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Human,
        },
    }
}

/// Detect language from file extension or content heuristics.
fn detect_language_from_content(content: &str, lang_hint: &str) -> Language {
    match lang_hint {
        "rust" => Language::Rust,
        "python" => Language::Python,
        "typescript" => Language::TypeScript,
        _ => {
            // Auto-detect from content
            if content.contains("fn ") || content.contains("use std::")
                || content.contains("pub fn") || content.contains("let mut")
            {
                Language::Rust
            } else if content.contains("def ") || content.contains("import ")
                || content.contains("from ")
            {
                Language::Python
            } else if content.contains("function ") || content.contains("const ")
                || content.contains("interface ") || content.contains("=>")
            {
                Language::TypeScript
            } else {
                Language::Rust // default fallback
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_enforce(
    project: &str,
    enforce: bool,
    from_stdin: bool,
    strict: bool,
    require_coverage: Option<u8>,
    output_format: &str,
    lang: &str,
    refs_override: Option<&str>,
    global_refs_override: Option<&str>,
) -> anyhow::Result<()> {
    let config = build_enforce_config_from_args(
        enforce,
        strict,
        require_coverage,
        from_stdin,
        output_format,
    );
    config.validate().map_err(|e| anyhow::anyhow!(e))?;

    let (content, display_path) = read_source_input(
        if from_stdin { None } else { Some(project) },
        from_stdin,
    )?;

    let language = detect_language_from_content(&content, lang);

    // Load reference files
    let ref_files = load_ref_files_for_enforce(
        project, refs_override, global_refs_override, language,
    )?;

    // Write content to temp file for checker
    let issues = run_checker_on_content(
        &content, &display_path, &ref_files, language,
    )?;

    // Build enforce result
    let mut result = build_enforce_result(&issues, &config);

    // Compute coverage if relevant
    let src_ctx = polyref::source_context::build_source_context(&content);
    let cov_report = coverage::compute_coverage(&src_ctx, &ref_files);
    result.coverage_pct = Some(cov_report.coverage_pct);

    // Check coverage gate
    if let Some(reason) = coverage::check_coverage_gate(&cov_report, &config) {
        result.verdict = polyref::enforce::EnforceVerdict::Blocked;
        let cov_issue = polyref::enforce::SerializedIssue {
            kind: "coverage_gate".to_string(),
            line: 0,
            message: reason,
            suggestion: None,
            similarity: None,
        };
        result.issues.push(cov_issue);
        result.issue_count = result.issues.len();
    }

    let output = format_enforce_result(&result, &config.output_format);
    println!("{}", output);

    if result.verdict == EnforceVerdict::Blocked {
        std::process::exit(1);
    }

    Ok(())
}

fn load_ref_files_for_enforce(
    project: &str,
    refs_override: Option<&str>,
    global_refs_override: Option<&str>,
    language: Language,
) -> anyhow::Result<Vec<polyref::generate::ReferenceFile>> {
    let refs_dir = if let Some(dir) = refs_override {
        std::path::PathBuf::from(dir)
    } else {
        let project_root = Path::new(project);
        let cfg = Config::load(project_root).unwrap_or_default();
        cfg.resolved_refs_dir()
    };

    let global_refs = global_refs_override.map(std::path::PathBuf::from);
    let global_refs_path = global_refs.as_deref();

    load_refs_from_dir(&refs_dir, global_refs_path, language)
}

fn load_refs_from_dir(
    refs_dir: &Path,
    global_refs_path: Option<&Path>,
    language: Language,
) -> anyhow::Result<Vec<polyref::generate::ReferenceFile>> {
    let mut ref_files = Vec::new();
    let lang_subdir = match language {
        Language::Rust => "rust",
        Language::Python => "python",
        Language::TypeScript => "typescript",
    };

    // Load from project refs dir
    let lang_dir = refs_dir.join(lang_subdir);
    if lang_dir.exists() {
        load_refs_from_language_dir(&lang_dir, language, &mut ref_files)?;
    }

    // Load from global refs dir (flat layout)
    if let Some(global_dir) = global_refs_path {
        if global_dir.exists() {
            load_refs_from_language_dir(global_dir, language, &mut ref_files)?;
        }
    }

    Ok(ref_files)
}

fn load_refs_from_language_dir(
    dir: &Path,
    language: Language,
    ref_files: &mut Vec<polyref::generate::ReferenceFile>,
) -> anyhow::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        let lib_name = extract_lib_name_from_path(&path);
        let version = extract_version_from_content(&content);
        let parsed_entries = parse_ref_entries(&content, language);
        ref_files.push(polyref::generate::ReferenceFile {
            library_name: lib_name,
            version,
            language,
            entries: parsed_entries,
            raw_content: content,
            file_path: path,
        });
    }
    Ok(())
}

/// Parse reference file entries using both v1 and v2 parsers, merging results.
fn parse_ref_entries(
    content: &str,
    language: Language,
) -> Vec<polyref::generate::ReferenceEntry> {
    match language {
        Language::Rust => {
            let v1 = polyref::generate::rust::parse_rust_reference(content);
            let v2 = polyref::ref_parser_v2::parse_reference_file_v2(content);
            merge_entries(v1, v2)
        }
        _ => polyref::ref_parser_v2::parse_reference_file_v2(content),
    }
}

/// Merge v1 and v2 entry lists. When both have an entry with the same name,
/// prefer the one with richer context (type_context set, or more fields).
fn merge_entries(
    v1: Vec<polyref::generate::ReferenceEntry>,
    v2: Vec<polyref::generate::ReferenceEntry>,
) -> Vec<polyref::generate::ReferenceEntry> {
    let mut result = Vec::new();
    let mut seen: std::collections::HashSet<(String, Option<String>)> =
        std::collections::HashSet::new();

    // v2 entries have better type_context from impl blocks — add them first
    for entry in &v2 {
        let key = (entry.name.clone(), entry.type_context.clone());
        if seen.insert(key) {
            result.push(entry.clone());
        }
    }

    // Add v1 entries not already covered
    for entry in &v1 {
        let key = (entry.name.clone(), entry.type_context.clone());
        if seen.insert(key) {
            result.push(entry.clone());
        }
    }

    result
}

fn extract_lib_name_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let name = stem
        .strip_prefix("lib_")
        .unwrap_or(stem)
        .replace(".polyref", "");
    // Also strip known extensions from stem (e.g. "tokio.polyref" → "tokio")
    name.split('.').next().unwrap_or(&name).to_string()
}

fn extract_version_from_content(content: &str) -> String {
    for line in content.lines().take(10) {
        let trimmed = line.trim().trim_start_matches("//").trim();
        if let Some(rest) = trimmed.strip_prefix("Version:") {
            return rest.trim().to_string();
        }
        if let Some(rest) = trimmed.strip_prefix("version:") {
            return rest.trim().to_string();
        }
    }
    "unknown".to_string()
}

fn run_checker_on_content(
    content: &str,
    display_path: &str,
    ref_files: &[polyref::generate::ReferenceFile],
    language: Language,
) -> anyhow::Result<Vec<polyref::check::Issue>> {
    // Write to a temp file so the checker can process it
    let ext = match language {
        Language::Rust => "rs",
        Language::Python => "py",
        Language::TypeScript => "ts",
    };

    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("polyref_enforce_{}.{}", std::process::id(), ext));
    std::fs::write(&tmp_path, content)?;

    let result = match language {
        Language::Rust => {
            let checker = polyref::check::rust::RustChecker;
            checker.check(std::slice::from_ref(&tmp_path), ref_files)?
        }
        Language::Python => {
            let checker = polyref::check::python::PythonChecker;
            checker.check(std::slice::from_ref(&tmp_path), ref_files)?
        }
        Language::TypeScript => {
            let checker = polyref::check::typescript::TypeScriptChecker;
            checker.check(std::slice::from_ref(&tmp_path), ref_files)?
        }
    };

    // Rewrite file paths in issues to display_path
    let issues: Vec<polyref::check::Issue> = result
        .issues
        .into_iter()
        .map(|mut issue| {
            issue.file = std::path::PathBuf::from(display_path);
            issue
        })
        .collect();

    let _ = std::fs::remove_file(&tmp_path);
    Ok(issues)
}

fn find_source_files(root: &Path, languages: &[Language]) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    let walker = walkdir::WalkDir::new(root)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in walker {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Skip generated refs and build dirs
        let path_str = path.to_string_lossy();
        if path_str.contains("refs") || path_str.contains("target") || path_str.contains("node_modules") {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "rs" if languages.contains(&Language::Rust) => files.push(path.to_path_buf()),
            "py" if languages.contains(&Language::Python) => files.push(path.to_path_buf()),
            "ts" | "tsx" if languages.contains(&Language::TypeScript) => {
                files.push(path.to_path_buf())
            }
            _ => {}
        }
    }

    files
}
