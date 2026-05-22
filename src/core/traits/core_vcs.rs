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
use std::sync::Arc;
use ymir::errors::Outcome;
use ymir::types::wallet::fafnir::VcEntry;

#[async_trait]
pub trait CoreVcsTrait: Send + Sync + 'static {
    fn repo(&self) -> Arc<dyn RepoTrait>;
    async fn get_all(&self) -> Outcome<Vec<VcEntry>> {
        self.repo().vcs().get_all().await
    }

    async fn get_by_id(&self, id: String) -> Outcome<VcEntry> {
        self.repo().vcs().get_by_id(&id).await
    }

    async fn delete(&self, id: String) -> Outcome<()> {
        self.repo().vcs().delete(&id).await
    }
}
