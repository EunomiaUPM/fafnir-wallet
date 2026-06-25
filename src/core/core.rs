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

use super::OrchestratorTrait;
use crate::config::CoreWalletConfig;
use crate::modules::{DidManagerModule, KeyManagerModule, Oid4VcModule, VcManagerModule};
use crate::services::oid4vci::Oid4vciTrait;
use crate::services::oid4vp::Oid4vpTrait;
use crate::services::repo::RepoTrait;
use crate::services::{HasOid4Vci, HasOid4Vp, HasRepo};
use std::sync::Arc;
use ymir::services::identity::{IdentityManager, IdentityTrait};
use ymir::services::vault::VaultService;
use ymir::services::{HasIdentity, HasVault};

pub struct CoreWallet {
    identity: Arc<IdentityManager>,
    repo: Arc<dyn RepoTrait>,
    oid4vci: Arc<dyn Oid4vciTrait>,
    oid4vp: Arc<dyn Oid4vpTrait>,
    vault: Arc<VaultService>,
    config: CoreWalletConfig,
}

impl CoreWallet {
    pub fn new(
        identity: Arc<IdentityManager>,
        repo: Arc<dyn RepoTrait>,
        oid4vci: Arc<dyn Oid4vciTrait>,
        oid4vp: Arc<dyn Oid4vpTrait>,
        vault: Arc<VaultService>,
        config: CoreWalletConfig,
    ) -> Self {
        Self {
            identity,
            repo,
            oid4vci,
            oid4vp,
            vault,
            config,
        }
    }
}

impl HasVault for CoreWallet {
    fn vault(&self) -> Arc<VaultService> {
        self.vault.clone()
    }
}

impl HasRepo for CoreWallet {
    fn repo(&self) -> Arc<dyn RepoTrait> {
        self.repo.clone()
    }
}

impl HasOid4Vp for CoreWallet {
    fn oid4vp(&self) -> Arc<dyn Oid4vpTrait> {
        self.oid4vp.clone()
    }
}

impl HasOid4Vci for CoreWallet {
    fn oid4vci(&self) -> Arc<dyn Oid4vciTrait> {
        self.oid4vci.clone()
    }
}

impl HasIdentity for CoreWallet {
    fn identity(&self) -> Arc<dyn IdentityTrait> {
        self.identity.clone()
    }
}

impl KeyManagerModule for CoreWallet {}

impl DidManagerModule for CoreWallet {}

impl VcManagerModule for CoreWallet {}

impl Oid4VcModule for CoreWallet {}

impl OrchestratorTrait for CoreWallet {
    fn config(&self) -> &CoreWalletConfig {
        &self.config
    }
}
