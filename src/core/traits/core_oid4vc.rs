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

use crate::services::repo::RepoTrait;
use async_trait::async_trait;
use futures::future::try_join_all;
use rayon::prelude::*;
use std::sync::Arc;
use ymir::errors::{Errors, Outcome};
use ymir::types::keys::Key;
use ymir::types::wallet::fafnir::{DidEntry, DidEntryReq, PreDidEntry};

#[async_trait]
pub trait CoreDidsTrait: Send + Sync + 'static {
    fn repo(&self) -> Arc<dyn RepoTrait>;
    
}
