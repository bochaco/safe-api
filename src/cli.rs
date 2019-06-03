// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use log::{debug, warn};
use std::io::{self, stdin, stdout, Write};
use structopt::StructOpt;

use crate::subcommands::keys::KeysSubCommands;
use crate::subcommands::mutable_data::MutableDataSubCommands;
use crate::subcommands::wallet::WalletSubCommands;
use crate::subcommands::SubCommands;
use safe_cli::{BlsKeyPair, Safe};
use unwrap::unwrap;

#[derive(StructOpt, Debug)]
/// Interact with the SAFE Network
#[structopt(raw(global_settings = "&[structopt::clap::AppSettings::ColoredHelp]"))]
struct CmdArgs {
    /// The safe:// address of target data
    #[structopt(short = "t", long = "target", raw(global = "true"))]
    target: Option<String>,
    /// The account's Root Container address
    #[structopt(long = "root", raw(global = "true"))]
    root: bool,
    /// Output data serlialisation
    #[structopt(short = "o", long = "output", raw(global = "true"))]
    output: Option<String>,
    /// Print human readable responses. (Alias to --output human-readable.)
    #[structopt(short = "hr", long = "human-readable", raw(global = "true"))]
    human: bool,
    /// Increase output verbosity. (More logs!)
    #[structopt(short = "v", long = "verbose", raw(global = "true"))]
    verbose: bool,
    /// Enable to query the output via SPARQL eg.
    #[structopt(short = "q", long = "query", raw(global = "true"))]
    query: Option<String>,
    /// Dry run of command. No data will be written. No coins spent.
    #[structopt(long = "dry-run", raw(global = "true"))]
    dry: bool,
    /// subcommands
    #[structopt(subcommand)]
    cmd: SubCommands,
}

pub fn run() -> Result<(), String> {
    // Let's first get all the arguments passed in
    let args = CmdArgs::from_args();

    let mut safe = Safe::new();

    debug!("Processing command: {:?}", args);

    match args.cmd {
        SubCommands::Keys { cmd } => {
            // Is it a create subcommand?
            match cmd {
                Some(KeysSubCommands::Create {
                    anon,
                    preload,
                    pk,
                    from,
                    test_coins,
                    ..
                }) => {
                    // Want an anonymous Key?
                    if anon {
                        create_new_key(&mut safe, test_coins, from, preload, pk);
                        println!("This was not linked from any container.");
                    } else {
                        // TODO: create Key and add it to the provided --target Wallet
                        println!("Missing --target or --anon");
                    }
                }
                Some(KeysSubCommands::Balance {}) => {
                    let sk = String::from(
                        "391987fd429b4718a59b165b5799eaae2e56c697eb94670de8886f8fb7387058",
                    ); // FIXME: get sk from args or from the account
                    let target = get_target_location(args.target)?;
                    let current_balance = safe.keys_balance_from_xorname(&target, &sk);
                    println!("Key's current balance: {}", current_balance);
                }
                Some(KeysSubCommands::Add { .. }) => println!("keys add ...coming soon!"),
                None => return Err("Missing keys sub-command. Use --help for details.".to_string()),
            };
        }
        SubCommands::MutableData { cmd } => {
            match cmd {
                Some(MutableDataSubCommands::Create {
                    name,
                    permissions,
                    tag,
                    sequenced,
                }) => {
                    let xorurl = safe.md_create(name, tag, permissions, sequenced);
                    println!("MutableData created at: {}", xorurl);
                }
                _ => return Err("Missing mutable-data subcommand".to_string()),
            };
        }
        SubCommands::Wallet { cmd } => {
            match cmd {
                Some(WalletSubCommands::Create {}) => {
                    let xorname = safe.wallet_create();
                    println!("Wallet created at XOR name: \"{}\"", xorname);
                }
                Some(WalletSubCommands::Balance {}) => {
                    let sk = String::from(
                        "391987fd429b4718a59b165b5799eaae2e56c697eb94670de8886f8fb7387058",
                    ); // FIXME: get sk from args or from the account
                    let target = get_target_location(args.target)?;
                    let balance = safe.wallet_balance(&target, &sk);
                    println!(
                        "Wallet at XOR name \"{}\" has a total balance of {} safecoins",
                        target, balance
                    );
                }
                Some(WalletSubCommands::Add {
                    preload,
                    from,
                    test_coins,
                    link,
                    name,
                    default,
                }) => {
                    let target = get_target_location(args.target)?;
                    let (xorname, key_pair) = match link {
                        Some(linked_key) => {
                            // Get pk from Key, and prompt user for the corresponding sk
                            let sk = prompt_user(
                                &format!(
                                    "Enter secret key corresponding to public key at XOR name \"{}\": ",
                                    linked_key
                                ),
                                "Invalid input",
                            );
                            let pk = safe.keys_fetch_pk(&linked_key, &sk);
                            println!(
                                "Spendable balance added with name '{}' in wallet located at XOR name \"{}\"",
                                name, target
                            );
                            (linked_key, Some(BlsKeyPair { pk, sk }))
                        }
                        None => {
                            let new_key =
                                create_new_key(&mut safe, test_coins, from, preload, None);
                            println!("New spendable balance generated with name '{}' in wallet located at XOR name \"{}\"", name, target);
                            new_key
                        }
                    };
                    safe.wallet_add(&target, &name, default, &unwrap!(key_pair), &xorname);
                }
                _ => return Err("Sub-command not supported yet".to_string()),
            };
        }
        _ => return Err("Command not supported yet".to_string()),
    };

    Ok(())
}

fn get_target_location(target_arg: Option<String>) -> Result<String, String> {
    match target_arg {
        Some(t) => Ok(t),
        None => {
            // try reading target from stdin then
            println!("Reading target from STDIN...");
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(n) => {
                    debug!(
                        "Read ({} bytes) from STDIN for target location: {}",
                        n, input
                    );
                    input.truncate(input.len() - 1);
                    Ok(input)
                }
                Err(_) => Err("There is no `--target` specified and no STDIN stream".to_string()),
            }
        }
    }
}

fn prompt_user(prompt_msg: &str, error_msg: &str) -> String {
    let mut user_input = String::new();
    print!("{}", prompt_msg);
    let _ = stdout().flush();
    stdin().read_line(&mut user_input).expect(error_msg);
    if let Some('\n') = user_input.chars().next_back() {
        user_input.pop();
    }
    if let Some('\r') = user_input.chars().next_back() {
        user_input.pop();
    }

    user_input
}

fn create_new_key(
    safe: &mut Safe,
    test_coins: bool,
    from: Option<String>,
    preload: Option<String>,
    pk: Option<String>,
) -> (String, Option<BlsKeyPair>) {
    // '--from' is either a Wallet XOR-URL, a Key XOR-URL, or a pk
    let from_key_pair = match from {
        Some(from_xorname) => {
            // TODO: support Key XOR-URL and pk, we now support only Key XOR name
            // Prompt the user for the secret key since 'from' is a Key and not a Wallet
            let sk = prompt_user(
                &format!(
                    "Enter secret key corresponding to public key at XOR name \"{}\": ",
                    from_xorname
                ),
                "Invalid input",
            );

            let pk = safe.keys_fetch_pk(&from_xorname, &sk);
            Some(BlsKeyPair { pk, sk })
        }
        None => None,
    };

    let (xorname, key_pair) = if test_coins {
        /*if cfg!(not(feature = "mock-network")) {
            warn!("Ignoring \"--test-coins\" flag since it's only available for \"mock-network\" feature");
            println!("Ignoring \"--test-coins\" flag since it's only available for \"mock-network\" feature");
            safe.keys_create(from, preload, pk)
        } else {*/
        warn!("Note that the Key to be created will be preloaded with **test coins** rather than real coins");
        println!("Note that the Key to be created will be preloaded with **test coins** rather than real coins");
        let amount = preload.unwrap_or("0".to_string());
        safe.keys_create_test_coins(amount, pk)
    // }
    } else {
        safe.keys_create(from_key_pair, preload, pk)
    };

    println!("New Key created at XOR name: \"{}\"", xorname);
    if let Some(pair) = &key_pair {
        println!("Key pair generated: pk=\"{}\", sk=\"{}\"", pair.pk, pair.sk);
    }

    (xorname, key_pair)
}
