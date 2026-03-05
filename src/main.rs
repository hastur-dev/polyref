use clap::Parser;
use polyref::check::Checker;
use polyref::config::Config;
use polyref::detect::Language;
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
        }) => polyref::commands::enforce::cmd_enforce(
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

    let source_files = find_source_files(project_root, &detected.languages);
    if verbose {
        println!("Found {} source files to check", source_files.len());
    }

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

    print_results(&results, output_format)?;

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
