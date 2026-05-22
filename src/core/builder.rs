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

use crate::core::Core;
use crate::services::repo::{DiskRepo, GlobalRepo, RepoTrait};

pub struct CoreBuilder;

impl CoreBuilder {
    pub async fn from_data_dir(data_dir: &Path) -> Outcome<Core> {
        let disk = DiskRepo::new(data_dir.to_path_buf()).await?;
        let repo: Arc<dyn RepoTrait> = Arc::new(GlobalRepo::Disk(disk));
        Ok(Core::new(repo))
    }
}
