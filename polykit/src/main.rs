mod commands;
mod formatting;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use polykit_core::release::BumpType;
use polykit_core::Scanner;
use tracing::Level;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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

    #[arg(long, action)]
    no_cache: bool,

    #[arg(long, action)]
    no_stream: bool,

    #[arg(long, action)]
    show_cache_stats: bool,

    #[arg(long)]
    remote_cache: Option<String>,

    #[arg(long)]
    remote_cache_url: Option<String>,

    #[arg(long, action)]
    remote_cache_readonly: bool,

    #[arg(long, action)]
    no_remote_cache: bool,
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
    Watch {
        task: String,
        packages: Vec<String>,
        #[arg(long)]
        debounce: Option<u64>,
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

    let scanner = if cli.no_cache {
        Scanner::new(&cli.packages_dir)
    } else {
        Scanner::with_default_cache(&cli.packages_dir)
    };
    let workspace_config = scanner.workspace_config();

    match cli.command {
        Commands::Scan { json } => {
            commands::cmd_scan(cli.packages_dir, json, cli.no_cache, cli.show_cache_stats)?
        }
        Commands::Graph { json } => {
            commands::cmd_graph(cli.packages_dir, json, cli.no_cache, cli.show_cache_stats)?
        }
        Commands::Affected { files, git, base } => commands::cmd_affected(
            cli.packages_dir,
            files,
            git,
            base,
            cli.no_cache,
            cli.show_cache_stats,
        )?,
        Commands::Build {
            packages,
            parallel,
            continue_on_error,
        } => {
            let parallel = parallel.or_else(|| workspace_config.and_then(|wc| wc.default_parallel));
            commands::cmd_build(
                cli.packages_dir,
                packages,
                parallel,
                continue_on_error,
                cli.no_cache,
                cli.no_stream,
                cli.show_cache_stats,
                cli.remote_cache_url,
                cli.remote_cache_readonly,
                cli.no_remote_cache,
            )?
        }
        Commands::Test {
            packages,
            parallel,
            continue_on_error,
        } => {
            let parallel = parallel.or_else(|| workspace_config.and_then(|wc| wc.default_parallel));
            commands::cmd_test(
                cli.packages_dir,
                packages,
                parallel,
                continue_on_error,
                cli.no_cache,
                cli.no_stream,
                cli.show_cache_stats,
                cli.remote_cache_url,
                cli.remote_cache_readonly,
                cli.no_remote_cache,
            )?
        }
        Commands::Release {
            package,
            bump,
            dry_run,
        } => commands::cmd_release(
            cli.packages_dir,
            package,
            bump.into(),
            dry_run,
            cli.no_cache,
            cli.show_cache_stats,
        )?,
        Commands::Why { package } => commands::cmd_why(
            cli.packages_dir,
            package,
            cli.no_cache,
            cli.show_cache_stats,
        )?,
        Commands::Validate { json } => {
            commands::cmd_validate(cli.packages_dir, json, cli.no_cache, cli.show_cache_stats)?
        }
        Commands::List { json } => {
            commands::cmd_list(cli.packages_dir, json, cli.no_cache, cli.show_cache_stats)?
        }
        Commands::Watch {
            task,
            packages,
            debounce,
        } => commands::cmd_watch(cli.packages_dir, task, packages, debounce, cli.no_cache)?,
    }

    Ok(())
}
