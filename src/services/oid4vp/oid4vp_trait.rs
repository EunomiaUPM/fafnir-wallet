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
use ymir::types::vcs::VPDef;
use ymir::types::wallet::fafnir::{DidEntry, KeyEntry, VcEntry};

/// Operaciones atómicas del lado holder en OID4VP.
///
/// Este service NO toca el repo. Las VCs y el holder (did/key) llegan
/// ya cargados desde el `CoreOid4vcTrait`. `KeyEntry` se mueve porque
/// el service necesita convertirlo en `Key` para firmar.
#[async_trait]
pub trait Oid4vpTrait: Send + Sync + 'static {
    /// Parsear/fetch de la presentation-request URI → `VPDef`.
    async fn resolve_request(&self, uri: &str) -> Outcome<VPDef>;

    /// Matching local: dadas las VCs disponibles del repo, devolver
    /// los ids que satisfacen cada input_descriptor de la VPDef.
    fn match_vcs(&self, vpd: &VPDef, available_vcs: &[VcEntry]) -> Outcome<Vec<String>>;

    /// Construir VP firmada y enviarla al verifier.
    /// Devuelve la `redirect_uri` si el verifier la incluye.
    async fn present(
        &self,
        uri: &str,
        vpd: &VPDef,
        selected_vcs: &[VcEntry],
        holder_did: &DidEntry,
        holder_key: KeyEntry,
    ) -> Outcome<Option<String>>;
}
