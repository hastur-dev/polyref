use clap::Parser;
use polyref_drift::config::DriftConfig;
use polyref_drift::{drift_detector, reporter};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "polyref-drift", version, about = "Detect version drift in polyref reference files")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "drift-config.toml")]
    config: String,

    /// Reference files directory (overrides config)
    #[arg(short, long)]
    refs: Option<String>,

    /// Output format: terminal or json
    #[arg(short, long)]
    output: Option<String>,

    /// Skip specific libraries
    #[arg(long)]
    skip: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config_path = Path::new(&cli.config);
    let mut config = DriftConfig::load_or_default(config_path);

    // Apply CLI overrides
    if let Some(refs) = &cli.refs {
        config.refs_dirs = vec![refs.clone()];
    }
    if let Some(output) = &cli.output {
        config.output = output.clone();
    }
    for skip in &cli.skip {
        if !config.skip.contains(skip) {
            config.skip.push(skip.clone());
        }
    }

    config.validate()?;

    let results = drift_detector::detect_drift(&config)?;
    reporter::report(&results, &config.output);

    if results.iter().any(|r| r.has_drift) {
        std::process::exit(1);
    }

    Ok(())
}
