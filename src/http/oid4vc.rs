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

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use ymir::errors::AppResult;
use ymir::types::issuing::VCCredOffer;
use ymir::types::vcs::VPDef;

use crate::core::traits::CoreOid4vcTrait;

/// Body de los endpoints que reciben una URI OIDC opaca.
#[derive(Debug, Deserialize)]
pub struct UriPayload {
    pub uri: String,
}

/// Endpoints atómicos OIDC del lado holder. El consumidor (p.ej. un
/// `FafnirService` en ymir) los compone para implementar `WalletTrait`.
pub struct Oid4vcRouter {
    core: Arc<dyn CoreOid4vcTrait>,
}

impl Oid4vcRouter {
    pub fn new(core: Arc<dyn CoreOid4vcTrait>) -> Self {
        Self { core }
    }

    pub fn router(self) -> Router {
        Router::new()
            // Endpoints "todo-en-uno" — los que el FafnirService llama
            // por defecto. Una sola petición HTTP completa el flujo.
            .route("/oid4vci", post(Self::process_oid4vci))
            .route("/oid4vp", post(Self::process_oid4vp))
            // Endpoints atómicos — útiles si un cliente quiere mostrar
            // pasos intermedios (offer, VPDef, matching) al usuario
            // antes de aceptar o firmar.
            .route("/vci/resolve_offer", post(Self::vci_resolve_offer))
            .route("/vci/accept_offer", post(Self::vci_accept_offer))
            .route("/vp/resolve_request", post(Self::vp_resolve_request))
            .route("/vp/present", post(Self::vp_present))
            .with_state(self.core)
    }

    /// POST /oid4vci  body: `{ "uri": "..." }`
    /// Flujo completo: resolver offer → token → proof → credential → guardar VC.
    async fn process_oid4vci(
        State(core): State<Arc<dyn CoreOid4vcTrait>>,
        Json(payload): Json<UriPayload>,
    ) -> AppResult<StatusCode> {
        core.process_oid4vci(&payload.uri).await?;
        Ok(StatusCode::OK)
    }

    /// POST /oid4vp  body: `{ "uri": "..." }`
    /// Flujo completo: resolver request → match → firmar VP → enviar al verifier.
    async fn process_oid4vp(
        State(core): State<Arc<dyn CoreOid4vcTrait>>,
        Json(payload): Json<UriPayload>,
    ) -> AppResult {
        let res = core.process_oid4vp(&payload.uri).await?;
        Ok(match res {
            Some(url) => Json(url).into_response(),
            None => StatusCode::OK.into_response(),
        })
    }

    /// POST /vci/resolve_offer  body: `{ "uri": "..." }`
    async fn vci_resolve_offer(
        State(core): State<Arc<dyn CoreOid4vcTrait>>,
        Json(payload): Json<UriPayload>,
    ) -> AppResult<Json<VCCredOffer>> {
        Ok(Json(core.vci_resolve_offer(&payload.uri).await?))
    }

    /// POST /vci/accept_offer  body: `VCCredOffer`
    /// Habla con el issuer, recibe la VC y la guarda. Devuelve 201.
    async fn vci_accept_offer(
        State(core): State<Arc<dyn CoreOid4vcTrait>>,
        Json(offer): Json<VCCredOffer>,
    ) -> AppResult<StatusCode> {
        core.vci_accept_offer(offer).await?;
        Ok(StatusCode::CREATED)
    }

    /// POST /vp/resolve_request  body: `{ "uri": "..." }`
    async fn vp_resolve_request(
        State(core): State<Arc<dyn CoreOid4vcTrait>>,
        Json(payload): Json<UriPayload>,
    ) -> AppResult<Json<VPDef>> {
        Ok(Json(core.vp_resolve_request(&payload.uri).await?))
    }

    /// POST /vp/present  body: `{ "uri": "..." }`
    /// Resuelve la request, matchea, firma y envía la VP. Devuelve la
    /// redirect URI si la hay (200 con string JSON) o 200 vacío.
    async fn vp_present(
        State(core): State<Arc<dyn CoreOid4vcTrait>>,
        Json(payload): Json<UriPayload>,
    ) -> AppResult {
        let res = core.vp_present(&payload.uri).await?;
        Ok(match res {
            Some(url) => Json(url).into_response(),
            None => StatusCode::OK.into_response(),
        })
    }
}
