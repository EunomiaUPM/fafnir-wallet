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
use ymir::config::traits::HostsConfigTrait;
use ymir::config::types::CommonHostsConfig;
use ymir::errors::{Errors, Outcome};
use ymir::utils::read;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CoreWalletConfig {
    hosts: CommonHostsConfig,
    path: PathBuf,
    /// Si `true`, las URLs externas (issuer/verifier) con host
    /// `127.0.0.1` o `localhost` se reescriben a `host.docker.internal`
    /// antes de hacer la petición HTTP. Útil cuando la wallet corre
    /// dentro de un contenedor Docker.
    #[serde(default)]
    docker: bool,
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

impl CoreWalletConfig {
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// `true` si la wallet debe reescribir `127.0.0.1` / `localhost`
    /// → `host.docker.internal` en las URLs externas.
    pub fn is_docker(&self) -> bool {
        self.docker
    }
}
