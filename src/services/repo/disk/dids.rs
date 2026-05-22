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

use crate::services::repo::subtraits::{BasicRepoTrait, DidsTrait};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use ymir::capabilities::Did;
use ymir::errors::{Errors, Outcome};
use ymir::types::dids::DidDocument;
use ymir::types::wallet::fafnir::{DidEntry, PreDidEntry};
use ymir::utils::HasId;

pub struct DidsOnDisk {
    dir: PathBuf,
}

impl DidsOnDisk {
    pub async fn new(dir: PathBuf) -> Outcome<Self> {
        tokio::fs::create_dir_all(&dir).await.map_err(|e| {
            Errors::write(
                dir.display().to_string(),
                "create_dir_all dids failed",
                Some(Box::new(e)),
            )
        })?;
        Ok(Self { dir })
    }
}

#[async_trait]
impl BasicRepoTrait<DidEntry, PreDidEntry> for DidsOnDisk {
    fn dir(&self) -> &Path {
        &self.dir
    }

    async fn build_entry(&self, req: PreDidEntry) -> Outcome<DidEntry> {
        let did = Did::create(req.r#type)?;
        let did_document = DidDocument::base(&did, &req.keys);

        let did = DidEntry {
            id: Uuid::new_v4().to_string(),
            did: did.id().to_string(),
            alias: req.alias,
            default: false,
            r#type: did.r#type(),
            keys: req.keys_id,
            did_document,
        };
        Ok(did)
    }
}

impl DidsTrait for DidsOnDisk {}
