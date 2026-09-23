mod runtime;
mod updater;

use std::{path::PathBuf, process::ExitCode, str::FromStr};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use runtime::{InstanceName, Runtime};

#[derive(Parser)]
#[command(
    name = "ths",
    version,
    about = "A one-command Zakura regtest environment"
)]
struct Cli {
    /// Isolated environment name.
    #[arg(long, global = true, default_value = "default")]
    name: InstanceName,
    /// Print machine-readable output where supported.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start an environment in the foreground; interrupting deletes it.
    Start {
        #[arg(long)]
        no_open: bool,
    },
    /// Build the runtime images from the current source.
    Build {
        /// Keep workspace Rust code unoptimized while optimizing dependencies.
        #[arg(long)]
        dev: bool,
    },
    /// Pull the exact runtime images for this launcher version.
    Pull,
    /// Check for or install a released launcher version.
    Update {
        /// Exact version to install, including an intentional rollback.
        #[arg(value_name = "VERSION", conflicts_with = "check")]
        version: Option<String>,
        /// Only report whether a newer stable release is available.
        #[arg(long)]
        check: bool,
    },
    /// Remove this installed launcher executable.
    Uninstall,
    /// Show service and endpoint status.
    Status,
    /// Open the dashboard in the default browser.
    Open,
    /// Print endpoints for developer tooling.
    Endpoints,
    /// Mine blocks on a running development environment.
    Mine {
        /// Number of blocks to mine.
        #[arg(value_parser = clap::value_parser!(u32).range(1..=10_000))]
        blocks: u32,
    },
    /// Send disposable Regtest ZEC to a unified or transparent address.
    Faucet {
        /// Regtest unified or transparent destination address.
        address: String,
        /// Amount of ZEC to send (maximum 5, up to 8 decimal places).
        #[arg(long, default_value = "1")]
        amount: ZecAmount,
    },
    /// Fund or move balances across development accounts without the dashboard.
    Deploy {
        #[command(subcommand)]
        action: DeployCommand,
    },
    /// Stream or print service logs.
    Logs {
        #[arg(value_parser = ["app", "zakura", "lightwalletd"])]
        service: Option<String>,
        #[arg(short, long)]
        follow: bool,
    },
    /// Stop and delete an environment.
    Stop,
    /// Delete one environment and all of its volumes.
    Reset {
        #[arg(long)]
        force: bool,
    },
    /// List known environments.
    List,
    /// Check local Docker and configuration prerequisites.
    Doctor,
}

#[derive(Subcommand)]
enum DeployCommand {
    /// Send faucet funds from the treasury to one or more accounts.
    Faucet {
        /// Account indices to fund (1-5), e.g. --accounts 1,2,3,5.
        #[arg(
            long,
            required = true,
            value_delimiter = ',',
            value_parser = clap::value_parser!(u8).range(1..=5)
        )]
        accounts: Vec<u8>,
        /// Amount of ZEC to send to each account (maximum 5, up to 8 decimal places).
        #[arg(long, default_value = "1")]
        amount: ZecAmount,
        /// Pool to fund.
        #[arg(long, value_enum, default_value = "orchard")]
        pool: Pool,
    },
    /// Send funds from one account to another.
    Send {
        /// Source account index (1-5).
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
        from: u8,
        /// Destination account index (1-5).
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
        to: u8,
        /// Amount of ZEC to send, up to 8 decimal places.
        #[arg(long)]
        amount: SendAmount,
        /// Pool to spend from.
        #[arg(long = "source-pool", value_enum, default_value = "orchard")]
        source_pool: Pool,
        /// Pool the destination account receives into.
        #[arg(long = "destination-pool", value_enum, default_value = "orchard")]
        destination_pool: Pool,
    },
    /// Move an account's transparent balance into its shielded (orchard) balance.
    Shield {
        /// Account index to shield from (1-5).
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
        from: u8,
        /// Account index to shield into (1-5); defaults to --from.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
        to: Option<u8>,
        /// Amount of ZEC to shield, up to 8 decimal places.
        #[arg(long)]
        amount: SendAmount,
    },
    /// Move an account's shielded (orchard) balance into its transparent balance.
    Unshield {
        /// Account index to unshield from (1-5).
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
        from: u8,
        /// Account index to unshield into (1-5); defaults to --from.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
        to: Option<u8>,
        /// Amount of ZEC to unshield, up to 8 decimal places.
        #[arg(long)]
        amount: SendAmount,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "lower")]
enum Pool {
    Orchard,
    Transparent,
}

impl Pool {
    fn as_str(self) -> &'static str {
        match self {
            Pool::Orchard => "orchard",
            Pool::Transparent => "transparent",
        }
    }
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    if let Some(Command::Update { version, check }) = &cli.command {
        return updater::run(version.as_deref(), *check, cli.json);
    }
    if matches!(cli.command, Some(Command::Uninstall)) {
        return updater::uninstall();
    }
    if should_check_for_updates(&cli)
        && let Some(notice) = updater::startup_notice()
    {
        println!("{notice}");
    }
    let runtime = Runtime::discover()?;
    match cli.command.unwrap_or(Command::Start { no_open: false }) {
        Command::Start { no_open } => runtime.start(&cli.name, no_open, cli.json),
        Command::Build { dev } => runtime.build(dev),
        Command::Pull => runtime.pull(),
        Command::Update { .. } => unreachable!("update is handled before runtime discovery"),
        Command::Uninstall => unreachable!("uninstall is handled before runtime discovery"),
        Command::Status => runtime.status(&cli.name, cli.json),
        Command::Open => runtime.open(&cli.name),
        Command::Endpoints => runtime.endpoints(&cli.name, cli.json),
        Command::Mine { blocks } => runtime.mine(&cli.name, blocks, cli.json),
        Command::Faucet { address, amount } => {
            runtime.faucet(&cli.name, &address, amount.zatoshi(), cli.json)
        }
        Command::Deploy { action } => match action {
            DeployCommand::Faucet {
                accounts,
                amount,
                pool,
            } => runtime.deploy_faucet(
                &cli.name,
                &accounts,
                amount.zatoshi(),
                pool.as_str(),
                cli.json,
            ),
            DeployCommand::Send {
                from,
                to,
                amount,
                source_pool,
                destination_pool,
            } => runtime.deploy_send(
                &cli.name,
                from,
                to,
                source_pool.as_str(),
                destination_pool.as_str(),
                amount.zatoshi(),
                cli.json,
            ),
            DeployCommand::Shield { from, to, amount } => runtime.deploy_send(
                &cli.name,
                from,
                to.unwrap_or(from),
                Pool::Transparent.as_str(),
                Pool::Orchard.as_str(),
                amount.zatoshi(),
                cli.json,
            ),
            DeployCommand::Unshield { from, to, amount } => runtime.deploy_send(
                &cli.name,
                from,
                to.unwrap_or(from),
                Pool::Orchard.as_str(),
                Pool::Transparent.as_str(),
                amount.zatoshi(),
                cli.json,
            ),
        },
        Command::Logs { service, follow } => runtime.logs(&cli.name, service.as_deref(), follow),
        Command::Stop => runtime.stop(&cli.name),
        Command::Reset { force } => runtime.reset(&cli.name, force),
        Command::List => runtime.list(cli.json),
        Command::Doctor => runtime.doctor(cli.json),
    }?;
    Ok(ExitCode::SUCCESS)
}

fn should_check_for_updates(cli: &Cli) -> bool {
    !cli.json && matches!(cli.command, None | Some(Command::Start { .. }))
}

fn _assert_pathbuf_send(_: PathBuf) {}

fn parse_zec_zatoshi(value: &str) -> Result<u64> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 8
    {
        bail!("amount must be a decimal ZEC value with at most 8 decimal places");
    }
    let whole = whole.parse::<u64>()?;
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>()? * 10u64.pow(8 - fraction.len() as u32)
    };
    whole
        .checked_mul(100_000_000)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| anyhow::anyhow!("amount is too large"))
}

/// A ZEC amount limited to 5, matching the server's per-request faucet cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ZecAmount(u64);

impl ZecAmount {
    fn zatoshi(self) -> u64 {
        self.0
    }
}

impl FromStr for ZecAmount {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let zatoshi = parse_zec_zatoshi(value)?;
        if zatoshi == 0 || zatoshi > 500_000_000 {
            bail!("amount must be greater than zero and no more than 5 ZEC");
        }
        Ok(Self(zatoshi))
    }
}

/// A ZEC amount with no upper bound, for transfers between accounts that are
/// only limited by the sending account's balance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SendAmount(u64);

impl SendAmount {
    fn zatoshi(self) -> u64 {
        self.0
    }
}

impl FromStr for SendAmount {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let zatoshi = parse_zec_zatoshi(value)?;
        if zatoshi == 0 {
            bail!("amount must be greater than zero");
        }
        Ok(Self(zatoshi))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_building_from_starting() {
        let default = Cli::try_parse_from(["ths"]).unwrap();
        assert!(default.command.is_none());

        let cli = Cli::try_parse_from(["ths", "build", "--dev"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Build { dev: true })));

        let cli = Cli::try_parse_from(["ths", "pull"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Pull)));

        let cli = Cli::try_parse_from(["ths", "update", "v1.2.3"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Update {
                version: Some(_),
                check: false
            })
        ));

        let cli = Cli::try_parse_from(["ths", "uninstall"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Uninstall)));

        let cli = Cli::try_parse_from(["ths", "mine", "10"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Mine { blocks: 10 })));

        let cli = Cli::try_parse_from(["ths", "--name", "alice", "mine", "3"]).unwrap();
        assert_eq!(cli.name.to_string(), "alice");
        assert!(matches!(cli.command, Some(Command::Mine { blocks: 3 })));

        let cli = Cli::try_parse_from(["ths", "mine", "3", "--name", "alice"]).unwrap();
        assert_eq!(cli.name.to_string(), "alice");

        assert!(Cli::try_parse_from(["ths", "mine", "0"]).is_err());
        assert!(Cli::try_parse_from(["ths", "mine", "10001"]).is_err());

        let cli = Cli::try_parse_from(["ths", "faucet", "uregtest1example"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Faucet {
                amount: ZecAmount(100_000_000),
                ..
            })
        ));

        let cli = Cli::try_parse_from([
            "ths",
            "faucet",
            "tmExample",
            "--amount",
            "1.25",
            "--name",
            "alice",
        ])
        .unwrap();
        assert_eq!(cli.name.to_string(), "alice");
        assert!(matches!(
            cli.command,
            Some(Command::Faucet {
                amount: ZecAmount(125_000_000),
                ..
            })
        ));

        assert!(Cli::try_parse_from(["ths", "update", "1.2.3", "--check"]).is_err());

        assert!(Cli::try_parse_from(["ths", "start", "--build"]).is_err());
    }

    #[test]
    fn checks_for_updates_only_during_human_readable_startup() {
        let default = Cli::try_parse_from(["ths"]).unwrap();
        assert!(should_check_for_updates(&default));

        let start = Cli::try_parse_from(["ths", "start", "--no-open"]).unwrap();
        assert!(should_check_for_updates(&start));

        let json = Cli::try_parse_from(["ths", "--json"]).unwrap();
        assert!(!should_check_for_updates(&json));

        let status = Cli::try_parse_from(["ths", "status"]).unwrap();
        assert!(!should_check_for_updates(&status));
    }

    #[test]
    fn parses_exact_zec_amounts() {
        assert_eq!("0.00000001".parse::<ZecAmount>().unwrap().zatoshi(), 1);
        assert_eq!("5".parse::<ZecAmount>().unwrap().zatoshi(), 500_000_000);
        for invalid in ["0", "5.00000001", "1.000000001", "-1", "1e2", ".5"] {
            assert!(invalid.parse::<ZecAmount>().is_err(), "accepted {invalid}");
        }
    }
}
