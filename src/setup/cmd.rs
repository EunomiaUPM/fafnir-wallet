/*
 * Copyright (C) 2026 - Universidad Politécnica de Madrid - UPM
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::cmp::PartialEq;

use crate::config::CoreWalletConfig;
use crate::setup::app::FafnirWallet;
use clap::{Parser, Subcommand};
use tracing::debug;
use ymir::errors::Outcome;

#[derive(Parser, Debug)]
#[command(name = "Rainbow Dataspace Authority Server")]
#[command(version = "0.1")]
struct WalletCli {
    #[command(subcommand)]
    command: WalletCliCommands,
}

#[derive(Parser, Debug, PartialEq)]
pub struct WalletCliArgs {
    #[arg(short, long)]
    env_file: String,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum WalletCliCommands {
    Start(WalletCliArgs),
    // Setup(WalletCliArgs),
}

pub struct WalletCommands;

impl WalletCommands {
    pub async fn init_command_line() -> Outcome<()> {
        debug!("Init the command line application");
        let cli = WalletCli::parse();

        match cli.command {
            WalletCliCommands::Start(args) => {
                let config = CoreWalletConfig::load(args.env_file)?;
                // let (config, vault) = Self::bootstrap(args)?;
                FafnirWallet::run(&config).await?
            } // WalletCliCommands::Setup(args) => {
              //     let (config, vault) = Self::bootstrap(args)?;
              //     match config.is_prod() {
              //         true => vault.write_all_secrets(None).await?,
              //         false => vault.write_local_secrets(None).await?,
              //     }
              //     let db_connection = vault.get_db_connection(&config).await;
              //     AuthorityMigration::run(&db_connection).await?;
              // }
        }

        Ok(())
    }

    // fn bootstrap(args: WalletCliArgs) -> Outcome<(CoreApplicationConfig, VaultService)> {
    //     let config = CoreApplicationConfig::load(args.env_file)?;
    //     let vault = if config.is_vault_real() {
    //         VaultService::Real(RealVaultService::new())
    //     } else {
    //         VaultService::Fake(FakeVaultService::new())
    //     };
    //     let table = json_to_table::json_to_table(&serde_json::to_value(&config)?)
    //         .collapse()
    //         .to_string();
    //     info!("Current Fafnir Config Config:\n{}", table);
    //     Ok((config, vault))
    // }
}
