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

use std::sync::Arc;

use crate::config::CoreWalletConfig;
use crate::core::CoreBuilder;
use crate::http::WalletRouter;
use axum::{Router, serve};
use tokio::net::TcpListener;
use tracing::info;
use ymir::config::traits::HostsConfigTrait;
use ymir::config::types::HostType;
use ymir::errors::{Errors, Outcome};

pub struct FafnirWallet;

impl FafnirWallet {
    pub async fn create_router(config: &CoreWalletConfig) -> Router {
        let core = CoreBuilder::from_data_dir(config.path())
            .await
            .expect("Error creating Wallet Router");

        WalletRouter::new(Arc::new(core)).router()
    }

    pub async fn run(config: &CoreWalletConfig) -> Outcome<()> {
        let router = Self::create_router(config).await;

        let port = config.get_internal_port(HostType::Http);
        let server_message = format!("Starting Fafnir Wallet in {}", port);
        info!("{}", server_message);

        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .map_err(|e| Errors::crazy("Error with tcp listener", Some(Box::new(e))))?;

        serve(listener, router)
            .await
            .map_err(|e| Errors::crazy("Error while running basic server", Some(Box::new(e))))
    }
}
