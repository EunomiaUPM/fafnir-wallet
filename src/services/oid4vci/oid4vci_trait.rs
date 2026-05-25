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
use ymir::errors::Outcome;
use ymir::types::issuing::VCCredOffer;
use ymir::types::wallet::fafnir::{DidEntry, KeyEntry, VcEntryReq};

/// Operaciones atómicas del lado holder en OID4VCI.
///
/// Este service NO toca el repo. Los modelos llegan ya cargados desde
/// el `CoreOid4vcTrait`. `KeyEntry` se mueve porque el service necesita
/// convertirlo en `Key` (la conversión consume el `KeyEntry`).
#[async_trait]
pub trait Oid4vciTrait: Send + Sync + 'static {
    /// Parsear/fetch de una credential-offer URI → estructura tipada.
    async fn resolve_offer(&self, uri: &str) -> Outcome<VCCredOffer>;

    /// Dialogar con el issuer remoto para conseguir la VC.
    /// Devuelve `VcEntryReq` listo para que el core lo guarde en el repo.
    async fn accept_offer(
        &self,
        offer: &VCCredOffer,
        holder_did: &DidEntry,
        holder_key: KeyEntry,
    ) -> Outcome<VcEntryReq>;
}
