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

use serde::{Deserialize, Serialize};

use std::path::PathBuf;
use tracing::debug;
use ymir::config::traits::{DatabaseConfigTrait, HostsConfigTrait};
use ymir::config::types::{CommonHostsConfig, DatabaseConfig};
use ymir::errors::{Errors, Outcome};
use ymir::utils::read;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CoreWalletConfig {
    hosts: CommonHostsConfig,
    db: DatabaseConfig,
    #[serde(default)]
    docker: bool,
    is_vault_real: bool,
}

impl CoreWalletConfig {
    pub fn load(env_file: String) -> Outcome<Self> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(env_file);
        debug!("Config file path: {}", path.display());

        let data = read(path)?;
        serde_norway::from_str(&data)
            .map_err(|e| Errors::parse("Unable to parse config file", Some(Box::new(e))))
    }
}

impl HostsConfigTrait for CoreWalletConfig {
    fn hosts(&self) -> &CommonHostsConfig {
        &self.hosts
    }
}

impl DatabaseConfigTrait for CoreWalletConfig {
    fn db(&self) -> &DatabaseConfig {
        &self.db
    }
}

impl CoreWalletConfig {
    pub fn is_docker(&self) -> bool {
        self.docker
    }
    pub fn is_vault_real(&self) -> bool {
        self.is_vault_real
    }
}
