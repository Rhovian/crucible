use clap::Parser;

fn main() -> anyhow::Result<()> {
    crucible_fuzz_cli::run(crucible_fuzz_cli::Cli::parse())
}
