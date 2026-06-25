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

use crate::config::CoreWalletConfig;
use crate::setup::app::FafnirWallet;
use crate::setup::db_migrations::WalletMigration;
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::debug;
use ymir::errors::{Errors, Outcome};
use ymir::services::vault::fake_vault::FakeVaultService;
use ymir::services::vault::vault_rs::RealVaultService;
use ymir::services::vault::{VaultService, VaultTrait};
use ymir::utils::{expect_from_env, read_json};

#[derive(Parser, Debug)]
#[command(name = "Fafnir Wallet")]
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
    Setup(WalletCliArgs),
}

pub struct WalletCommands;

impl WalletCommands {
    pub async fn init_command_line() -> Outcome<()> {
        debug!("Init the command line application");
        let cli = WalletCli::parse();

        match cli.command {
            WalletCliCommands::Start(args) => {
                let config = CoreWalletConfig::load(args.env_file)?;
                let vault = if config.is_vault_real() {
                    Arc::new(VaultService::Real(RealVaultService::new()?))
                } else {
                    Arc::new(VaultService::Fake(FakeVaultService::new()?))
                };
                FafnirWallet::run(config, vault).await?
            }
            WalletCliCommands::Setup(args) => {
                let config = CoreWalletConfig::load(args.env_file)?;
                let vault = if config.is_vault_real() {
                    Arc::new(VaultService::Real(RealVaultService::new()?))
                } else {
                    Arc::new(VaultService::Fake(FakeVaultService::new()?))
                };

                let mut mapa = HashMap::new();
                let vault_path =
                    PathBuf::from(std::env::var("VAULT_PATH").map_err(|e| {
                        Errors::vault("VAULT_PATH env var not set", Some(Box::new(e)))
                    })?);

                let secret_path = vault_path.join("secrets");

                let vault_path = expect_from_env("VAULT_APP_DB");
                let db_json: Value = read_json(secret_path.join("db.json"))?;
                mapa.insert(vault_path, db_json);

                vault.write_all_secrets(Some(mapa)).await?;
                let db_connection = vault.get_db_connection(&config).await?;
                WalletMigration::run(&db_connection).await?;
            }
        }
        Ok(())
    }
}
