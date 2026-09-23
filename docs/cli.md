# `ths` CLI reference

`ths` is the Thus Spoke Zakura launcher. It manages the Docker-based Regtest
environment and, once an environment is running, talks to its dashboard API on
your behalf. This page documents every command in detail, with examples.

For a quick tour of the dashboard itself, see the [README](../README.md).

## Conventions used below

- All commands accept two **global options**, which can be placed before or
  after the subcommand:
  - `--name <NAME>` — target a named, isolated environment instead of the
    default one (default: `default`).
  - `--json` — print machine-readable JSON instead of human-readable text,
    where the command supports it.
- Accounts are referred to by **index**, `1` through `5`. Every environment
  starts with five disposable development accounts; there is no need to copy
  full Regtest addresses to use the commands on this page. (A hidden sixth
  account acts as the mining/faucet treasury and is not addressable from the
  CLI.)
- Amounts are ZEC decimal strings with up to 8 decimal places, e.g. `1`,
  `0.5`, `2.25000001`. These are disposable Regtest coins with no value.
- Every command in this reference (other than `start`, `build`, `pull`,
  `update`, `uninstall`, `list`, and `doctor`) requires a running environment.
  Start one first with `ths` (or `ths --name <NAME>`).

---

## Environment lifecycle

### `ths` / `ths start`

Starts a fresh environment in the foreground and opens the dashboard in your
browser. Interrupting with Ctrl+C stops the environment and deletes its chain,
wallet, keys, and Docker volumes.

```console
ths
ths start --no-open      # don't open a browser tab
ths --name alice         # start a second, independent environment named "alice"
```

### `ths build [--dev]`

Builds the runtime Docker images from the current source checkout (for
development from a clone, not needed for the installed release).

```console
ths build             # release-optimized images
ths build --dev       # keep workspace Rust code unoptimized for faster rebuilds
```

### `ths pull`

Pulls the exact runtime images that match this launcher's version, instead of
building them locally.

```console
ths pull
```

### `ths status [--json]`

Shows whether the named environment is running and prints its endpoints.

```console
ths status
ths status --name alice --json
```

### `ths open`

Opens the running dashboard in your default browser.

```console
ths open
```

### `ths endpoints [--json]`

Prints the dashboard, Zakura RPC, lightwalletd, and P2P endpoints, useful for
scripts and other developer tooling.

```console
ths endpoints
ths endpoints --json
```

### `ths logs [app|zakura|lightwalletd] [-f|--follow]`

Prints or streams logs for a service in the named environment. Defaults to the
`app` (dashboard/server) service.

```console
ths logs                       # last logs from the app service
ths logs zakura -f             # follow node logs
ths logs lightwalletd --follow # follow lightwalletd logs
```

### `ths stop`

Stops the named environment and deletes its containers, volumes, and network.

```console
ths stop
```

### `ths reset --force`

Same as `stop`, but named explicitly as a destructive action; requires
`--force` since it permanently deletes chain, wallet, and seed data.

```console
ths reset --force
```

### `ths list [--json]`

Lists every known environment and its dashboard URL.

```console
ths list
```

### `ths doctor [--json]`

Checks that Docker is reachable and prints the launcher's configuration
directory.

```console
ths doctor
```

### `ths update [VERSION] [--check]` / `ths uninstall`

Checks for, installs, or rolls back a released launcher version, or removes
the installed `ths` executable.

```console
ths update --check     # only report whether a newer release exists
ths update              # install the latest verified release
ths update v0.2.0       # install (or roll back to) an exact version
ths uninstall
```

### `ths mine <BLOCKS>`

Mines blocks on the running environment and synchronizes its wallet — useful
for testing confirmations, expiry, or coinbase maturity.

```console
ths mine 1
ths mine 10 --json
```

---

## Moving funds

There are two ways to move funds: the single-purpose `ths faucet <ADDRESS>`
command (send to an arbitrary Regtest address, no environment accounts
required), and the account-index-based `ths deploy ...` commands below, which
are built for scripting against the five development accounts without
clicking through the dashboard UI.

### `ths faucet <ADDRESS> [--amount <ZEC>]`

Sends disposable Regtest ZEC directly to a Regtest unified or transparent
address (not necessarily one of the five development accounts). Limited to 5
ZEC per request; defaults to 1 ZEC.

```console
ths faucet uregtest1exampleaddress...
ths faucet tmExampleTransparentAddress... --amount 2.5
```

## `ths deploy` — scripting the development accounts

`ths deploy` is a base command with four subcommands — `faucet`, `send`,
`shield`, and `unshield` — for funding and moving balances between the five
development accounts (index `1`-`5`) directly from a script or terminal,
instead of clicking through the dashboard's Faucet and Send dialogs.

Every `deploy` subcommand mines the confirming block automatically and prints
the resulting transaction ID (and, for `deploy send`/`shield`/`unshield`, the
confirming block hash). Pass `--json` at the top level for machine-readable
output.

### `ths deploy faucet --accounts <LIST> [--amount <ZEC>] [--pool <POOL>]`

Funds one or more accounts from the treasury faucet in a single command.

- `--accounts <LIST>` (required) — comma-separated account indices, e.g.
  `1,2,3,5`. Each listed account receives its own independent faucet
  transaction for the full `--amount`.
- `--amount <ZEC>` — amount to send to **each** account (default `1`, maximum
  `5`, matching the dashboard faucet's per-request limit).
- `--pool <orchard|transparent>` — which balance to fund (default `orchard`).

```console
# Fund accounts 1, 2, 3, and 5 with 3 ZEC each (shielded)
ths deploy faucet --accounts 1,2,3,5 --amount 3

# Fund a single account's transparent balance
ths deploy faucet --accounts 4 --amount 1 --pool transparent

# Machine-readable output, e.g. for a setup script
ths --json deploy faucet --accounts 1,2,3,4,5 --amount 5
```

If some accounts fail (for example, an out-of-range index or a treasury
shortfall) the command still funds the rest, prints a `Failed to fund ...`
line per failure, and exits with a non-zero status summarizing how many of the
requests failed.

### `ths deploy send --from <N> --to <N> --amount <ZEC> [--source-pool <POOL>] [--destination-pool <POOL>]`

Sends funds from one development account to another (or to itself, moving
between pools — see `shield`/`unshield` below for the common shortcuts).

- `--from <N>` / `--to <N>` (required) — account indices, `1`-`5`.
- `--amount <ZEC>` (required) — no upper limit beyond the sending account's
  balance.
- `--source-pool <orchard|transparent>` — pool to spend from (default
  `orchard`).
- `--destination-pool <orchard|transparent>` — pool the destination account
  receives into (default `orchard`).

```console
# Move 1 ZEC from account 2's shielded balance to account 3's shielded balance
ths deploy send --from 2 --to 3 --amount 1

# Send from account 1's transparent balance into account 4's shielded balance
ths deploy send --from 1 --to 4 --amount 0.5 --source-pool transparent --destination-pool orchard
```

### `ths deploy shield --from <N> [--to <N>] --amount <ZEC>`

Shorthand for `deploy send` with `--source-pool transparent
--destination-pool orchard`: moves an account's unshielded (transparent)
balance into its shielded (Orchard) balance.

- `--from <N>` (required) — account to shield from.
- `--to <N>` — account to shield into (default: same as `--from`).
- `--amount <ZEC>` (required).

```console
# Shield 0.5 ZEC of account 4's transparent balance into its own Orchard balance
ths deploy shield --from 4 --amount 0.5

# Shield into a different account's Orchard balance
ths deploy shield --from 4 --to 2 --amount 0.5
```

> Note: because transparent notes must be fully spent, the wallet may route
> any leftover change from a transparent source into the shielded pool as
> well — so shielding "0.5 ZEC" from an account holding 1 transparent ZEC can
> leave that account with slightly less than 0.5 ZEC left over.

### `ths deploy unshield --from <N> [--to <N>] --amount <ZEC>`

Shorthand for `deploy send` with `--source-pool orchard --destination-pool
transparent`: moves an account's shielded (Orchard) balance into its
unshielded (transparent) balance.

- `--from <N>` (required) — account to unshield from.
- `--to <N>` — account to unshield into (default: same as `--from`).
- `--amount <ZEC>` (required).

```console
# Unshield 0.2 ZEC from account 1's own balances
ths deploy unshield --from 1 --amount 0.2

# Unshield into a different account's transparent balance
ths deploy unshield --from 1 --to 3 --amount 0.2
```

### Putting it together: seeding a fresh environment

A typical setup script for a fresh environment might look like:

```console
ths --name demo --no-open &
sleep 5   # or poll `ths status --name demo` until it reports "running"

ths --name demo deploy faucet --accounts 1,2,3,4,5 --amount 5
ths --name demo deploy faucet --accounts 2 --amount 1 --pool transparent
ths --name demo deploy shield --from 2 --amount 0.5
ths --name demo deploy send --from 1 --to 3 --amount 1
```

This funds all five accounts, gives account 2 some transparent balance,
shields part of it, and moves shielded funds between accounts 1 and 3 — all
without opening the dashboard.

---

## Command summary

| Command | What it does |
| --- | --- |
| `ths` | Start a fresh environment and open the dashboard |
| `ths start --no-open` | Start without opening a browser |
| `ths build [--dev]` | Build runtime images from source |
| `ths pull` | Pull the exact images for this launcher version |
| `ths status [--json]` | Show health and endpoint information |
| `ths open` | Open the running dashboard |
| `ths endpoints [--json]` | Print endpoints for scripts and developer tools |
| `ths mine <N>` | Mine blocks and synchronize the wallet |
| `ths faucet <ADDRESS> [--amount]` | Send disposable ZEC to any Regtest address |
| `ths deploy faucet --accounts <LIST> [--amount] [--pool]` | Fund one or more of the five accounts by index |
| `ths deploy send --from --to --amount [--source-pool] [--destination-pool]` | Move funds between accounts/pools by index |
| `ths deploy shield --from [--to] --amount` | Move transparent balance into Orchard |
| `ths deploy unshield --from [--to] --amount` | Move Orchard balance into transparent |
| `ths logs [service] [-f]` | Stream or print service logs |
| `ths list` | List known environments |
| `ths stop` | Stop and delete the environment |
| `ths reset --force` | Force-delete one environment and all its data |
| `ths doctor` | Check Docker and local configuration |
| `ths update [--check]` | Check for or install a newer release |
| `ths uninstall` | Remove the installed launcher executable |

Every command accepts `--name` for isolated environments; see the
[README](../README.md#useful-commands) for more on running multiple named
environments side by side.
