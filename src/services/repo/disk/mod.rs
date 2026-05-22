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

//! Implementación on-disk de los repos.
//!
//! `DiskRepo` agrupa los tres sub-repos (DIDs, keys, VCs) cada uno apuntando
//! a un subdirectorio bajo el `base_path` (`<base>/dids`, `<base>/keys`,
//! `<base>/vcs`). Equivale al `RepoForSql` de heimdall pero con filesystem
//! en lugar de Postgres.

mod dids;
mod keys;
mod vcs;

use std::path::PathBuf;
use std::sync::Arc;

use ymir::errors::{Errors, Outcome};

use super::subtraits::{DidsTrait, KeysTrait, VcsTrait};

pub use dids::DidsOnDisk;
pub use keys::KeysOnDisk;
pub use vcs::VcsOnDisk;

#[derive(Clone)]
pub struct DiskRepo {
    dids: Arc<DidsOnDisk>,
    keys: Arc<KeysOnDisk>,
    vcs: Arc<VcsOnDisk>,
}

impl DiskRepo {
    /// Crea el repo bajo `base_path`. Garantiza la existencia de los tres
    /// subdirectorios.
    pub async fn new(base_path: PathBuf) -> Outcome<Self> {
        tokio::fs::create_dir_all(&base_path).await.map_err(|e| {
            Errors::write(
                base_path.display().to_string(),
                "create_dir_all base failed",
                Some(Box::new(e)),
            )
        })?;
        let dids = Arc::new(DidsOnDisk::new(base_path.join("dids")).await?);
        let keys = Arc::new(KeysOnDisk::new(base_path.join("keys")).await?);
        let vcs = Arc::new(VcsOnDisk::new(base_path.join("vcs")).await?);
        Ok(Self { dids, keys, vcs })
    }

    pub fn dids(&self) -> Arc<dyn DidsTrait> {
        self.dids.clone()
    }
    pub fn keys(&self) -> Arc<dyn KeysTrait> {
        self.keys.clone()
    }
    pub fn vcs(&self) -> Arc<dyn VcsTrait> {
        self.vcs.clone()
    }
}
