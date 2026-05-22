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

use crate::services::repo::RepoTrait;
use async_trait::async_trait;
use futures::future::try_join_all;
use rayon::prelude::*;
use std::sync::Arc;
use ymir::errors::{Errors, Outcome};
use ymir::types::keys::Key;
use ymir::types::wallet::fafnir::{DidEntry, DidEntryReq, PreDidEntry};

#[async_trait]
pub trait CoreDidsTrait: Send + Sync + 'static {
    fn repo(&self) -> Arc<dyn RepoTrait>;
    async fn get_all(&self) -> Outcome<Vec<DidEntry>> {
        self.repo().dids().get_all().await
    }

    async fn get_by_id(&self, id: String) -> Outcome<DidEntry> {
        self.repo().dids().get_by_id(&id).await
    }

    async fn create(&self, entry: DidEntryReq) -> Outcome<DidEntry> {
        let keys_entry = try_join_all(
            entry
                .keys
                .iter()
                .map(|key_id| async move { self.repo().keys().get_by_id(key_id).await }),
        )
        .await?;

        let keys = keys_entry
            .into_par_iter()
            .map(|key| key.try_into())
            .collect::<Result<Vec<Key>, Errors>>()?;

        let pre_did = PreDidEntry {
            alias: entry.alias,
            r#type: entry.r#type,
            keys_id: entry.keys,
            keys,
            service: entry.service,
        };
        self.repo().dids().create(pre_did).await
    }

    async fn delete(&self, id: String) -> Outcome<()> {
        self.repo().dids().delete(&id).await
    }

    async fn set_default(&self, id: String) -> Outcome<()> {
        self.repo().dids().set_default(&id).await
    }
    async fn get_default(&self) -> Outcome<DidEntry> {
        self.repo().dids().get_default().await
    }
}
