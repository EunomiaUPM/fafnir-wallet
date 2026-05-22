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

use super::BasicRepoTrait;
use async_trait::async_trait;
use ymir::errors::{Errors, Outcome};
use ymir::types::wallet::fafnir::{DidEntry, PreDidEntry};

#[async_trait]
pub trait DidsTrait: BasicRepoTrait<DidEntry, PreDidEntry> {
    async fn get_default(&self) -> Outcome<DidEntry> {
        let all = self.get_all().await?;
        all.into_iter().find(|d| d.default).ok_or_else(|| {
            Errors::missing_resource("default", "no default DID set in wallet", None)
        })
    }

    async fn set_default(&self, id: &str) -> Outcome<()> {
        let all = self.get_all().await?;
        let mut found = false;
        for mut entry in all {
            let is_target = entry.id == id;
            if is_target {
                found = true;
            }
            let was = entry.default;
            let now = is_target;
            if was != now {
                entry.default = now;
                self.save(entry).await?;
            }
        }
        if !found {
            return Err(Errors::missing_resource(
                id,
                format!("did {id} not present in wallet"),
                None,
            ));
        }
        Ok(())
    }
}
