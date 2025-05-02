use clap::{Parser, Subcommand};
use snafu::{ResultExt, Snafu};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Build(BuildArgs),
    /// Run for CI
    Test,
    /// Run parallel test(Need cargo install nextest)
    NTest,
}

#[derive(clap::Args)]
struct BuildArgs {
    /// Use `release` (long compile but more optimization)
    #[clap(long)]
    release: bool,
    #[clap(long, value_enum, default_value_t = DestDir::Mo2)]
    dest_mode: DestDir,
    dest: Option<PathBuf>,
}

impl Default for BuildArgs {
    #[inline]
    fn default() -> Self {
        Self {
            release: false,
            dest_mode: DestDir::Root,
            dest: Default::default(),
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum DestDir {
    /// root Build directory.
    Root,
    /// D drive Mod Organizer 2 directory.
    Mo2,
}

impl DestDir {
    fn path(&self) -> &Path {
        match self {
            Self::Root => Path::new("./build/mods/wheeler_rs/SKSE/Plugins/"),
            Self::Mo2 => Path::new("D:/GAME/ModOrganizer Skyrim SE/mods/wheeler_rs/SKSE/Plugins/"),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => build_main_dll(BuildArgs::default()),
        Some(Commands::Build(args)) => build_main_dll(args),
        Some(Commands::Test) => run_command(
            "cargo",
            &[
                "test",
                "--workspace",
                "--features",
                "debug",
                "--no-default-features",
            ],
            Some("./test_results.txt"),
        ),
        Some(Commands::NTest) => run_command(
            "cargo",
            &[
                "nextest",
                "run",
                "--workspace",
                "--features",
                "test_on_local",
                "--no-default-features",
            ],
            Some("./test_results.txt"),
        ),
    }
}

fn build_main_dll(args: BuildArgs) -> Result<()> {
    println!("Building dll...");

    const DLL_NAME: &str = "wheeler_rs";

    if args.release {
        run_command(
            "cargo",
            &["build", "-p", DLL_NAME, "--features", "full", "--release"],
            None,
        )?;
    } else {
        run_command(
            "cargo",
            &["build", "-p", DLL_NAME, "--features", "full"],
            None,
        )?;
    };

    let dest_dir = args.dest.unwrap_or(args.dest_mode.path().to_path_buf());

    fs::create_dir_all(&dest_dir).context(CreateDirSnafu)?;

    let dll_filename = format!("{DLL_NAME}.dll");
    let pdb_filename = format!("{DLL_NAME}.pdb");
    let opt_mode = if args.release { "release" } else { "debug" };
    let binding = format!("./target/{opt_mode}/");
    let cargo_build_dir = Path::new(&binding);

    let dll_path = cargo_build_dir.join(&dll_filename);
    let pdb_path = cargo_build_dir.join(&pdb_filename);

    fs::copy(dll_path, dest_dir.join(dll_filename)).context(FileCopySnafu)?;
    fs::copy(pdb_path, dest_dir.join(pdb_filename)).context(FileCopySnafu)?;

    Ok(())
}

fn run_command(cmd: &str, args: &[&str], output_file: Option<&str>) -> Result<()> {
    println!("Running: {cmd} {}", args.join(" "));
    let output = Command::new(cmd)
        .args(args)
        .output()
        .context(CommandExecutionSnafu)?;

    if let Some(output_file) = output_file {
        fs::write(output_file, &output.stdout).context(FileWriteSnafu)?;
        fs::write(output_file, &output.stderr).context(FileWriteSnafu)?;
    } else {
        std::io::Write::write_all(&mut std::io::stdout(), &output.stdout)
            .context(FileWriteSnafu)?;
        std::io::Write::write_all(&mut std::io::stderr(), &output.stderr)
            .context(FileWriteSnafu)?;
    }

    if !output.status.success() {
        eprintln!("Command failed: {} {:?}", cmd, args);
    }

    Ok(())
}

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Failed to execute command: {}", source))]
    CommandExecution { source: std::io::Error },
    #[snafu(display("Failed to write to file: {}", source))]
    FileWrite { source: std::io::Error },
    #[snafu(display("Failed to create directory: {}", source))]
    CreateDir { source: std::io::Error },
    #[snafu(display("Failed to copy file: {}", source))]
    FileCopy { source: std::io::Error },
}
type Result<T, E = Error> = core::result::Result<T, E>;
