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
use crate::core::CoreWallet;
use crate::http::WalletRouter;
use crate::services::oid4vci::Oid4vciService;
use crate::services::oid4vp::{oid4vp_1_0, oid4vp_draft20};
use crate::services::repo::{RepoTrait, WalletRepo};
use axum::{Router, serve};
use tokio::net::TcpListener;
use tracing::info;
use ymir::config::traits::HostsConfigTrait;
use ymir::config::types::HostType;
use ymir::errors::{Errors, Outcome};
use ymir::services::identity::{IdentityManager, IdentityTrait};
use ymir::services::vault::{VaultService, VaultTrait};

pub struct FafnirWallet;

impl FafnirWallet {
    pub async fn create_router(
        config: CoreWalletConfig,
        vault: Arc<VaultService>,
    ) -> Outcome<Router> {
        let db_connection = vault.get_db_connection(&config).await?;
        let repo: Arc<dyn RepoTrait> = Arc::new(WalletRepo::new(db_connection));

        let identity_manager = match repo.dids().get_default().await {
            Ok(model) => Arc::new(IdentityManager::load(&model)?),
            Err(_) => Arc::new(IdentityManager::new()),
        };

        let oid4vci = Arc::new(Oid4vciService::new(
            identity_manager.get_identity(),
            vault.clone(),
            config.is_docker(),
        ));
        let oid4vp_draft20 = Arc::new(oid4vp_draft20::Oid4vpService::new(
            vault.clone(),
            config.is_docker(),
        ));
        let oid4vp_v1_0 = Arc::new(oid4vp_1_0::Oid4vpService::new(
            vault.clone(),
            config.is_docker(),
        ));

        let core = CoreWallet::new(
            identity_manager,
            repo,
            oid4vci,
            oid4vp_draft20,
            oid4vp_v1_0,
            vault,
            config,
        );
        let router = WalletRouter::new(Arc::new(core));
        Ok(router.router())
    }

    pub async fn run(config: CoreWalletConfig, vault: Arc<VaultService>) -> Outcome<()> {
        let port = config.get_internal_port(HostType::Http);

        let router = Self::create_router(config, vault.clone()).await?;

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
