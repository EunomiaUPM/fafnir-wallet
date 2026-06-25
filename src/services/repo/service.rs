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

use super::RepoTrait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use ymir::services::repo::postgres::wallet::{DidPostgresRepo, KeyPostgresRepo, VcPostgresRepo};
use ymir::services::repo::traits::wallet::{DidRepoTrait, KeyRepoTrait, VcRepoTrait};

pub struct WalletRepo {
    keys: Arc<dyn KeyRepoTrait>,
    dids: Arc<dyn DidRepoTrait>,
    vcs: Arc<dyn VcRepoTrait>,
}

impl WalletRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        let keys = Arc::new(KeyPostgresRepo::new(db.clone()));
        let dids = Arc::new(DidPostgresRepo::new(db.clone()));
        let vcs = Arc::new(VcPostgresRepo::new(db.clone()));
        Self { keys, dids, vcs }
    }
}

impl RepoTrait for WalletRepo {
    fn dids(&self) -> Arc<dyn DidRepoTrait> {
        self.dids.clone()
    }

    fn keys(&self) -> Arc<dyn KeyRepoTrait> {
        self.keys.clone()
    }

    fn vcs(&self) -> Arc<dyn VcRepoTrait> {
        self.vcs.clone()
    }
}
