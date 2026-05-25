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

use async_trait::async_trait;
use ymir::errors::{Errors, Outcome};
use ymir::types::issuing::VCCredOffer;
use ymir::types::vcs::VPDef;
use ymir::types::wallet::fafnir::{DidEntry, KeyEntry};

use crate::services::oid4vci::Oid4vciTrait;
use crate::services::oid4vp::Oid4vpTrait;
use crate::services::repo::RepoTrait;

/// Orquesta los flujos OID4VCI y OID4VP del lado holder.
///
/// Patrón ymir: los `Oid4vciTrait` / `Oid4vpTrait` son lógica pura +
/// HTTP externo; este trait carga modelos del repo, llama al service y
/// persiste el resultado. Las implementaciones (p.ej. `Core`) solo
/// deben aportar `repo()`, `oid4vci()` y `oid4vp()`.
#[async_trait]
pub trait CoreOid4vcTrait: Send + Sync + 'static {
    fn repo(&self) -> Arc<dyn RepoTrait>;
    fn oid4vci(&self) -> Arc<dyn Oid4vciTrait>;
    fn oid4vp(&self) -> Arc<dyn Oid4vpTrait>;

    // ───────────────────────────── OID4VCI ─────────────────────────────

    async fn vci_resolve_offer(&self, uri: &str) -> Outcome<VCCredOffer> {
        self.oid4vci().resolve_offer(uri).await
    }

    /// Aceptar un offer previamente resuelto: hablar con el issuer y
    /// guardar la VC obtenida en el repo.
    async fn vci_accept_offer(&self, offer: VCCredOffer) -> Outcome<()> {
        let (did, key_entry) = load_default_holder(self.repo()).await?;
        let vc_req = self
            .oid4vci()
            .accept_offer(&offer, &did, key_entry)
            .await?;
        self.repo().vcs().create(vc_req).await?;
        Ok(())
    }

    // ───────────────────────────── OID4VP ──────────────────────────────

    async fn vp_resolve_request(&self, uri: &str) -> Outcome<VPDef> {
        self.oid4vp().resolve_request(uri).await
    }

    /// Flujo completo: resolver request → match → presentar al verifier.
    /// Devuelve `redirect_uri` si la hubo.
    async fn vp_present(&self, uri: &str) -> Outcome<Option<String>> {
        let vpd = self.oid4vp().resolve_request(uri).await?;

        let available = self.repo().vcs().get_all().await?;
        let selected_ids = self.oid4vp().match_vcs(&vpd, &available)?;
        let selected: Vec<_> = available
            .into_iter()
            .filter(|v| selected_ids.iter().any(|id| id == &v.id))
            .collect();

        let (did, key_entry) = load_default_holder(self.repo()).await?;

        self.oid4vp()
            .present(uri, &vpd, &selected, &did, key_entry)
            .await
    }

    // ─────────────────────── ALTO NIVEL (todo-en-uno) ──────────────────
    //
    // Estos son los que se exponen como `POST /oid4vci` y `POST /oid4vp`
    // y los que llamará el `FafnirService` de ymir desde fuera. Hacen
    // todo el flujo en una sola petición HTTP.

    /// Desde una credential-offer URI: resolver + aceptar + guardar.
    async fn process_oid4vci(&self, uri: &str) -> Outcome<()> {
        let offer = self.oid4vci().resolve_offer(uri).await?;
        self.vci_accept_offer(offer).await
    }

    /// Desde una presentation-request URI: resolver + match + firmar + enviar.
    /// Devuelve `redirect_uri` si la hubo.
    async fn process_oid4vp(&self, uri: &str) -> Outcome<Option<String>> {
        self.vp_present(uri).await
    }
}

/// Carga el DID por defecto y el `KeyEntry` de su primera key.
/// Devolvemos `KeyEntry` (no `Key`) porque la conversión a `Key`
/// consume el `KeyEntry` y el service lo necesita en propiedad
/// para construir el `SigningCtx`.
async fn load_default_holder(repo: Arc<dyn RepoTrait>) -> Outcome<(DidEntry, KeyEntry)> {
    let did = repo.dids().get_default().await?;
    let key_id = did.keys.first().ok_or_else(|| {
        Errors::missing_resource(
            "default-did-key",
            format!("default DID {} has no keys", did.did),
            None,
        )
    })?;
    let key_entry = repo.keys().get_by_id(key_id).await?;
    Ok((did, key_entry))
}
