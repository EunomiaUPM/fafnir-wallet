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

use crate::services::repo::subtraits::{BasicRepoTrait, KeysTrait};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use ymir::errors::{Errors, Outcome};
use ymir::types::wallet::fafnir::{KeyEntry, KeyEntryReq};

pub struct KeysOnDisk {
    dir: PathBuf,
}

impl KeysOnDisk {
    pub async fn new(dir: PathBuf) -> Outcome<Self> {
        tokio::fs::create_dir_all(&dir).await.map_err(|e| {
            Errors::write(
                dir.display().to_string(),
                "create_dir_all keys failed",
                Some(Box::new(e)),
            )
        })?;
        Ok(Self { dir })
    }
}

#[async_trait]
impl BasicRepoTrait<KeyEntry, KeyEntryReq> for KeysOnDisk {
    fn dir(&self) -> &Path {
        &self.dir
    }

    async fn build_entry(&self, req: KeyEntryReq) -> Outcome<KeyEntry> {
        let key = KeyEntry {
            id: Uuid::new_v4().to_string(),
            alias: req.alias,
            kty: req.kty,
            crv: req.crv,
            pem: req.pem,
        };

        Ok(key)
    }
}

impl KeysTrait for KeysOnDisk {}
