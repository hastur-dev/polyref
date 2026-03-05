use clap::{Parser, Subcommand};
use polyref_gen::{dirs, python_gen, python_source_gen, rust_source_gen, rustdoc_gen, scanner, typescript_gen};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "polyref-gen", version, about = "Generate polyref reference files from documentation and source code")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a Rust reference from rustdoc JSON
    Rustdoc {
        /// Path to rustdoc JSON file
        #[arg(short, long)]
        input: String,
        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Generate a Rust reference by parsing source files directly
    RustSource {
        /// Path to Rust project directory (containing Cargo.toml)
        #[arg(short, long)]
        dir: String,
        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Generate a Python reference from .pyi stub files
    PythonStub {
        /// Path to .pyi stub file
        #[arg(short, long)]
        input: String,
        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Generate a Python reference by parsing source files directly
    PythonSource {
        /// Path to Python project directory
        #[arg(short, long)]
        dir: String,
        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Generate a TypeScript reference from .d.ts declaration files
    Typescript {
        /// Path to .d.ts declaration file
        #[arg(short, long)]
        input: String,
        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Scan directories for projects and generate references for all of them
    Scan {
        /// Directories to scan for projects (can specify multiple)
        #[arg(short, long, required = true, num_args = 1..)]
        dirs: Vec<String>,
        /// Output directory for generated reference files.
        /// Defaults to the OS temp directory (e.g. /tmp/polyref/refs
        /// on Linux, %TEMP%/polyref/refs on Windows).
        /// Override with POLYREF_DATA_DIR env var.
        #[arg(short, long)]
        output_dir: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Rustdoc { input, output } => {
            let doc = rustdoc_gen::parse_rustdoc_json(Path::new(&input))?;
            let content = rustdoc_gen::generate_ref_file(&doc);
            write_output(&content, output.as_deref())?;
        }
        Commands::RustSource { dir, output } => {
            let doc = rust_source_gen::parse_rust_project(Path::new(&dir))?;
            let content = rustdoc_gen::generate_ref_file(&doc);
            write_output(&content, output.as_deref())?;
        }
        Commands::PythonStub { input, output } => {
            let stub = python_gen::parse_pyi_stub(Path::new(&input))?;
            let content = python_gen::generate_polyref_file(&stub);
            write_output(&content, output.as_deref())?;
        }
        Commands::PythonSource { dir, output } => {
            let stub = python_source_gen::parse_python_project(Path::new(&dir))?;
            let content = python_gen::generate_polyref_file(&stub);
            write_output(&content, output.as_deref())?;
        }
        Commands::Typescript { input, output } => {
            let decl = typescript_gen::parse_dts_file(Path::new(&input))?;
            let content = typescript_gen::generate_polyref_file(&decl);
            write_output(&content, output.as_deref())?;
        }
        Commands::Scan { dirs, output_dir } => {
            let resolved_output_dir = match &output_dir {
                Some(d) => d.clone(),
                None => {
                    match dirs::default_refs_output_dir() {
                        Some(d) => d.to_string_lossy().to_string(),
                        None => {
                            eprintln!("Error: Could not determine default output directory.");
                            eprintln!("Set POLYREF_DATA_DIR or pass --output-dir explicitly.");
                            std::process::exit(1);
                        }
                    }
                }
            };
            let output_path = Path::new(&resolved_output_dir);
            let mut all_projects = Vec::new();

            for dir in &dirs {
                let dir_path = Path::new(dir);
                eprintln!("Scanning {}...", dir_path.display());
                match scanner::discover_projects(dir_path) {
                    Ok(projects) => {
                        eprintln!("  Found {} projects", projects.len());
                        for p in &projects {
                            eprintln!(
                                "    {:?}: {} ({})",
                                p.kind,
                                p.name,
                                p.path.display()
                            );
                        }
                        all_projects.extend(projects);
                    }
                    Err(e) => {
                        eprintln!("  Error scanning {}: {}", dir, e);
                    }
                }
            }

            if all_projects.is_empty() {
                eprintln!("No projects found.");
                return Ok(());
            }

            eprintln!(
                "\nGenerating references for {} projects into {}...",
                all_projects.len(),
                output_path.display()
            );

            let results = scanner::generate_all(&all_projects, output_path);

            let mut success_count = 0;
            let mut fail_count = 0;
            for result in &results {
                if result.success {
                    success_count += 1;
                    eprintln!(
                        "  OK: {} -> {}",
                        result.project.name,
                        result.output_path.display()
                    );
                } else {
                    fail_count += 1;
                    eprintln!(
                        "  FAIL: {} — {}",
                        result.project.name,
                        result.error.as_deref().unwrap_or("unknown error")
                    );
                }
            }

            eprintln!("\nDone: {} succeeded, {} failed", success_count, fail_count);

            if fail_count > 0 {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn write_output(content: &str, output: Option<&str>) -> anyhow::Result<()> {
    if let Some(path) = output {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        eprintln!("Wrote reference file to {}", path);
    } else {
        print!("{}", content);
    }
    Ok(())
}
