use clap::Parser;
use polyref_gen::rustdoc_gen;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "polyref-gen", version, about = "Generate polyref reference files from documentation")]
struct Cli {
    /// Path to rustdoc JSON file
    #[arg(short, long)]
    input: String,

    /// Output file path (stdout if not specified)
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let input_path = Path::new(&cli.input);

    let doc = rustdoc_gen::parse_rustdoc_json(input_path)?;
    let ref_content = rustdoc_gen::generate_ref_file(&doc);

    if let Some(output) = &cli.output {
        std::fs::write(output, &ref_content)?;
        eprintln!("Wrote reference file to {}", output);
    } else {
        print!("{}", ref_content);
    }

    Ok(())
}
