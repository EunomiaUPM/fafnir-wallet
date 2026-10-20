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

use super::repo::RepoTrait;
use crate::services::oid4vci::Oid4vciTrait;
use crate::services::oid4vp::Oid4vpTrait;
use std::sync::Arc;
use ymir::errors::Outcome;

pub trait HasRepo {
    fn repo(&self) -> Arc<dyn RepoTrait>;
}

pub trait HasOid4Vci {
    fn oid4vci(&self) -> Arc<dyn Oid4vciTrait>;
}

pub trait HasOid4Vp {
    /// The OpenID4VP version that handles `uri`.
    fn oid4vp(&self, uri: &str) -> Outcome<Arc<dyn Oid4vpTrait>>;
}

