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

//! Aggregator concreto que envuelve los backends.
//!
//! Patrón equivalente al `VaultService` de ymir:
//! - `GlobalRepo` es un enum con una variante por backend.
//! - Implementa [`RepoTrait`] delegando con `match`.
//! - Static dispatch entre backends, sin `Arc<dyn>` adicional.

use std::sync::Arc;

use crate::services::repo::disk::DiskRepo;
use crate::services::repo::repo_trait::RepoTrait;
use crate::services::repo::subtraits::{DidsTrait, KeysTrait, VcsTrait};

#[derive(Clone)]
pub enum GlobalRepo {
    Disk(DiskRepo),
    // Postgres(PostgresRepo),  // pendiente — añadí la variante cuando
    // implementes el backend en `services/repo/postgres/`.
}

impl RepoTrait for GlobalRepo {
    fn dids(&self) -> Arc<dyn DidsTrait> {
        match self {
            GlobalRepo::Disk(r) => r.dids(),
        }
    }
    fn keys(&self) -> Arc<dyn KeysTrait> {
        match self {
            GlobalRepo::Disk(r) => r.keys(),
        }
    }
    fn vcs(&self) -> Arc<dyn VcsTrait> {
        match self {
            GlobalRepo::Disk(r) => r.vcs(),
        }
    }
}
