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
use ymir::types::issuance::VcCredOffer;

#[async_trait]
pub trait Oid4vciTrait: Send + Sync + 'static {
    async fn resolve_offer(&self, uri: &str) -> Outcome<VcCredOffer>;
    async fn accept_offer(&self, offer: &VcCredOffer) -> Outcome<Vec<vc::Plan>>;
}
