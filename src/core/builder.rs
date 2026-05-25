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

use std::path::Path;
use std::sync::Arc;

use ymir::errors::Outcome;
use ymir::services::client::{ClientService, ClientTrait};

use crate::core::Core;
use crate::services::http::HostRewriteClient;
use crate::services::oid4vci::{Oid4vciService, Oid4vciTrait};
use crate::services::oid4vp::{Oid4vpService, Oid4vpTrait};
use crate::services::repo::{DiskRepo, GlobalRepo, RepoTrait};

pub struct CoreBuilder;

impl CoreBuilder {
    pub async fn from_data_dir(data_dir: &Path, docker: bool) -> Outcome<Core> {
        let disk = DiskRepo::new(data_dir.to_path_buf()).await?;
        let repo: Arc<dyn RepoTrait> = Arc::new(GlobalRepo::Disk(disk));

        let base: Arc<dyn ClientTrait> = Arc::new(ClientService::default());
        let client: Arc<dyn ClientTrait> = Arc::new(HostRewriteClient::new(base, docker));

        let oid4vci: Arc<dyn Oid4vciTrait> = Arc::new(Oid4vciService::new(client.clone()));
        let oid4vp: Arc<dyn Oid4vpTrait> = Arc::new(Oid4vpService::new(client.clone()));

        Ok(Core::new(repo, oid4vci, oid4vp))
    }
}
