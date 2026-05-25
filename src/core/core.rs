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

use std::sync::Arc;

use crate::core::traits::{CoreDidsTrait, CoreKeysTrait, CoreOid4vcTrait, CoreTrait, CoreVcsTrait};
use crate::services::oid4vci::Oid4vciTrait;
use crate::services::oid4vp::Oid4vpTrait;
use crate::services::repo::RepoTrait;

pub struct Core {
    repo: Arc<dyn RepoTrait>,
    oid4vci: Arc<dyn Oid4vciTrait>,
    oid4vp: Arc<dyn Oid4vpTrait>,
}

impl Core {
    pub fn new(
        repo: Arc<dyn RepoTrait>,
        oid4vci: Arc<dyn Oid4vciTrait>,
        oid4vp: Arc<dyn Oid4vpTrait>,
    ) -> Self {
        Self {
            repo,
            oid4vci,
            oid4vp,
        }
    }
}

impl CoreKeysTrait for Core {
    fn repo(&self) -> Arc<dyn RepoTrait> {
        self.repo.clone()
    }
}

impl CoreDidsTrait for Core {
    fn repo(&self) -> Arc<dyn RepoTrait> {
        self.repo.clone()
    }
}

impl CoreVcsTrait for Core {
    fn repo(&self) -> Arc<dyn RepoTrait> {
        self.repo.clone()
    }
}

impl CoreOid4vcTrait for Core {
    fn repo(&self) -> Arc<dyn RepoTrait> {
        self.repo.clone()
    }

    fn oid4vci(&self) -> Arc<dyn Oid4vciTrait> {
        self.oid4vci.clone()
    }

    fn oid4vp(&self) -> Arc<dyn Oid4vpTrait> {
        self.oid4vp.clone()
    }
}

impl CoreTrait for Core {}
