mod commands;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use polykit_core::release::BumpType;
use tracing::Level;

#[derive(Parser)]
#[command(name = "polykit")]
#[command(about = "Fast, language-agnostic monorepo orchestration tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "./packages")]
    packages_dir: PathBuf,

    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[arg(short, long, action)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    Scan {
        #[arg(long, action)]
        json: bool,
    },
    Graph {
        #[arg(long, action)]
        json: bool,
    },
    Affected {
        files: Vec<String>,
        #[arg(long)]
        git: bool,
        #[arg(long)]
        base: Option<String>,
    },
    Build {
        packages: Vec<String>,
        #[arg(short = 'j', long)]
        parallel: Option<usize>,
        #[arg(long, action)]
        continue_on_error: bool,
    },
    Test {
        packages: Vec<String>,
        #[arg(short = 'j', long)]
        parallel: Option<usize>,
        #[arg(long, action)]
        continue_on_error: bool,
    },
    Release {
        package: String,
        #[arg(long, value_enum, default_value = "patch")]
        bump: BumpArg,
        #[arg(long, action)]
        dry_run: bool,
    },
    Why {
        package: String,
    },
    Validate {
        #[arg(long, action)]
        json: bool,
    },
    List {
        #[arg(long, action)]
        json: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum BumpArg {
    Major,
    Minor,
    Patch,
}

impl From<BumpArg> for BumpType {
    fn from(arg: BumpArg) -> Self {
        match arg {
            BumpArg::Major => BumpType::Major,
            BumpArg::Minor => BumpType::Minor,
            BumpArg::Patch => BumpType::Patch,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.quiet {
        Level::ERROR
    } else {
        match cli.verbose {
            0 => Level::INFO,
            1 => Level::DEBUG,
            _ => Level::TRACE,
        }
    };

    tracing_subscriber::fmt().with_max_level(log_level).init();

    match cli.command {
        Commands::Scan { json } => commands::cmd_scan(cli.packages_dir, json)?,
        Commands::Graph { json } => commands::cmd_graph(cli.packages_dir, json)?,
        Commands::Affected { files, git, base } => {
            commands::cmd_affected(cli.packages_dir, files, git, base)?
        }
        Commands::Build {
            packages,
            parallel,
            continue_on_error,
        } => commands::cmd_build(cli.packages_dir, packages, parallel, continue_on_error)?,
        Commands::Test {
            packages,
            parallel,
            continue_on_error,
        } => commands::cmd_test(cli.packages_dir, packages, parallel, continue_on_error)?,
        Commands::Release {
            package,
            bump,
            dry_run,
        } => commands::cmd_release(cli.packages_dir, package, bump.into(), dry_run)?,
        Commands::Why { package } => commands::cmd_why(cli.packages_dir, package)?,
        Commands::Validate { json } => commands::cmd_validate(cli.packages_dir, json)?,
        Commands::List { json } => commands::cmd_list(cli.packages_dir, json)?,
    }

    Ok(())
}
