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

use async_trait::async_trait;
use ymir::data::entities::wallet::vc;
use ymir::errors::Outcome;
use ymir::types::dids::Did;
use ymir::types::vcs::VPDef;
use ymir::types::wallet::KeyRef;

#[async_trait]
pub trait Oid4vpTrait: Send + Sync + 'static {
    async fn resolve_request(&self, uri: &str) -> Outcome<VPDef>;

    async fn present(
        &self,
        uri: &str,
        vpd: &VPDef,
        selected_vcs: &[vc::Model],
        holder_did: &Did,
        holder_key: &KeyRef,
    ) -> Outcome<Option<String>>;
}
