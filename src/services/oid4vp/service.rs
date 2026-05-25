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

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use url::Url;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info, warn};
use uuid::Uuid;
use ymir::capabilities::{Did, Signer};
use ymir::errors::{BadFormat, Errors, Outcome};
use ymir::services::client::ClientTrait;
use ymir::types::http::Body;
use ymir::types::jwt::VPJwtClaims;
use ymir::types::keys::Key;
use ymir::types::vcs::VPDef;
use ymir::types::vps::VpDocument;
use ymir::types::wallet::fafnir::{DidEntry, KeyEntry, SigningCtx, VcBodyType, VcEntry};
use ymir::utils::{get_query_param, json_headers, ResponseExt};

use crate::services::oid4vp::Oid4vpTrait;

/// Implementación del flujo OID4VP desde el lado holder.
pub struct Oid4vpService {
    client: Arc<dyn ClientTrait>,
}

impl Oid4vpService {
    pub fn new(client: Arc<dyn ClientTrait>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Oid4vpTrait for Oid4vpService {
    async fn resolve_request(&self, uri: &str) -> Outcome<VPDef> {
        info!("OID4VP: resolve_request({})", uri);

        let parsed = parse_openid4vp_uri(uri)?;

        // by value: presentation_definition=<json>
        if let Some(json) = optional_query(&parsed, "presentation_definition") {
            debug!("OID4VP: VPDef by value");
            return serde_json::from_str(&json).map_err(|e| {
                Errors::parse(
                    "presentation_definition payload is not valid JSON",
                    Some(Box::new(e)),
                )
            });
        }

        // by reference: presentation_definition_uri=<url>
        if let Some(pd_uri) = optional_query(&parsed, "presentation_definition_uri") {
            debug!("OID4VP: VPDef by reference → {}", pd_uri);
            return self
                .client
                .get(&pd_uri, Some(json_headers()))
                .await?
                .parse_json()
                .await;
        }

        // request_uri=<url> apuntando a un JWT firmado con la
        // Authorization Request. Para la implementación base
        // soportamos solo el caso en que el JWT trae embebida la
        // presentation_definition en sus claims.
        if let Some(req_uri) = optional_query(&parsed, "request_uri") {
            debug!("OID4VP: Authorization Request JWT → {}", req_uri);
            let jwt_str = self
                .client
                .get(&req_uri, None)
                .await?
                .parse_text()
                .await?;
            let jwt = ymir::types::jwt::Jwt::parse(jwt_str)?;
            let payload: Value = jwt.payload().clone();
            let pd = payload.get("presentation_definition").ok_or_else(|| {
                Errors::format(
                    BadFormat::Received,
                    "request_uri JWT lacks presentation_definition",
                    None,
                )
            })?;
            return serde_json::from_value(pd.clone()).map_err(|e| {
                Errors::parse(
                    "presentation_definition in JWT not deserialisable",
                    Some(Box::new(e)),
                )
            });
        }

        Err(Errors::format(
            BadFormat::Received,
            "URI lacks presentation_definition / _uri / request_uri",
            None,
        ))
    }

    fn match_vcs(&self, vpd: &VPDef, available_vcs: &[VcEntry]) -> Outcome<Vec<String>> {
        info!(
            "OID4VP: match_vcs(vpd.id={}, n_vcs={})",
            vpd.id,
            available_vcs.len()
        );

        let mut chosen = Vec::with_capacity(vpd.input_descriptors.len());
        for descriptor in &vpd.input_descriptors {
            // En ymir el `id` del descriptor coincide con el vc_type
            // declarado (mira `InputDescriptor::new(vc_type, ...)`).
            let wanted = descriptor.id.as_str();

            let m = available_vcs
                .iter()
                .find(|vc| vc_matches_type(&vc.parsed_document, wanted))
                .ok_or_else(|| {
                    Errors::missing_resource(
                        wanted,
                        format!("no stored VC matches input descriptor '{}'", wanted),
                        None,
                    )
                })?;
            chosen.push(m.id.clone());
        }
        Ok(chosen)
    }

    async fn present(
        &self,
        uri: &str,
        vpd: &VPDef,
        selected_vcs: &[VcEntry],
        holder_did: &DidEntry,
        holder_key: KeyEntry,
    ) -> Outcome<Option<String>> {
        info!(
            "OID4VP: present(uri={}, vpd.id={}, n_vcs={}, holder={})",
            uri,
            vpd.id,
            selected_vcs.len(),
            holder_did.did
        );

        // 1) Parámetros que el verifier puso en la URI.
        let parsed = parse_openid4vp_uri(uri)?;
        let client_id = get_query_param(&parsed, "client_id")?;
        let nonce = get_query_param(&parsed, "nonce")?;
        let response_uri = get_query_param(&parsed, "response_uri")?;
        let state = optional_query(&parsed, "state");

        // 2) Extraer los JWT strings de las VCs seleccionadas. Para esta
        // implementación base solo admitimos VCs guardadas como JWT
        // (no `VcBodyType::Value` "raw" — embeber una VC JSON-LD en una VP
        // JWT requiere otra envoltura distinta).
        let vc_jwts: Vec<String> = selected_vcs
            .iter()
            .map(|vc| match &vc.r#type {
                VcBodyType::Jwt(s) => Ok(s.clone()),
                VcBodyType::Value(_) => Err(Errors::not_impl(
                    "Embedding a raw JSON VC in a JWT VP is not supported in the base impl",
                    None,
                )),
            })
            .collect::<Outcome<Vec<_>>>()?;

        // 3) Construir el VpDocument. Importante: el `id` de la VP debe
        // coincidir con el `id` de la VPDef (lo valida el verifier).
        let vp_doc = VpDocument {
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
            id: vpd.id.clone(),
            r#type: vec!["VerifiablePresentation".to_string()],
            holder: Some(holder_did.did.clone()),
            verifiable_credential: vc_jwts,
        };

        let now = Utc::now().timestamp();
        let claims = VPJwtClaims {
            aud: client_id.clone(),
            nonce: nonce.clone(),
            iss: Some(holder_did.did.clone()),
            sub: Some(holder_did.did.clone()),
            jti: Some(format!("urn:uuid:{}", Uuid::new_v4())),
            nbf: Some(now),
            exp: Some(now + 300),
            iat: Some(now),
            vp: vp_doc,
        };

        // 4) Firmar el vp_token como JWT enveloped.
        let key: Key = holder_key.try_into()?;
        let did = Did::parse_from_kid(&holder_did.did)?;
        let sig_ctx = SigningCtx::new(did, key);
        let vp_jwt = Signer::sign_enveloped(
            "vp+jwt",
            "vp+jwt",
            &serde_json::to_value(&claims)?,
            &sig_ctx,
        )?;

        // 5) Construir presentation_submission.
        let submission = build_submission(vpd);

        // 6) POST al response_uri (direct_post = form-urlencoded).
        let mut form = HashMap::new();
        form.insert("vp_token".to_string(), vp_jwt.as_str().to_string());
        form.insert(
            "presentation_submission".to_string(),
            serde_json::to_string(&submission)?,
        );
        if let Some(s) = state {
            form.insert("state".to_string(), s);
        }

        let resp = self
            .client
            .post(&response_uri, None, Body::Form(form))
            .await?;

        // 7) Algunos verifiers responden 200 con JSON {"redirect_uri": "..."}.
        // Otros responden 200 vacío. Lo tratamos tolerante: si el body
        // se parsea como RedirectResponse, devolvemos su uri.
        let body = resp.parse_text().await.unwrap_or_default();
        if body.trim().is_empty() {
            return Ok(None);
        }
        match serde_json::from_str::<RedirectResponse>(&body) {
            Ok(r) => Ok(Some(r.redirect_uri)),
            Err(_) => {
                warn!("Verifier response was not a RedirectResponse: {}", body);
                Ok(None)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct RedirectResponse {
    redirect_uri: String,
}

/// Parsea una URI tipo `openid4vp://...`. `url::Url::parse` necesita un
/// authority válido tras `://`; algunas wallets generan
/// `openid4vp://?...` (authority vacío) y `Url::parse` se queja. Para
/// esos casos normalizamos a `openid4vp://authorize?...`.
fn parse_openid4vp_uri(raw: &str) -> Outcome<Url> {
    let candidate = if raw.starts_with("openid4vp://?") {
        raw.replacen("openid4vp://?", "openid4vp://authorize?", 1)
    } else {
        raw.to_string()
    };
    Url::parse(&candidate).map_err(|e| {
        Errors::parse(
            format!("Invalid presentation request URI: {raw}"),
            Some(Box::new(e)),
        )
    })
}

fn optional_query(url: &Url, key: &str) -> Option<String> {
    get_query_param(url, key).ok()
}

/// Mira `$.vc.type` (W3C V1) o `$.type` (V2) sobre el parsed_document de
/// la VC y comprueba si contiene el tipo pedido.
fn vc_matches_type(doc: &Value, wanted: &str) -> bool {
    let types = doc
        .get("vc")
        .and_then(|v| v.get("type"))
        .or_else(|| doc.get("type"));
    match types {
        Some(Value::Array(arr)) => arr.iter().any(|t| t.as_str() == Some(wanted)),
        Some(Value::String(s)) => s == wanted,
        _ => false,
    }
}

/// Construye un presentation_submission mínimo: un descriptor_map por
/// cada input_descriptor, apuntando a la VC i-ésima del array
/// `verifiableCredential` del VP.
fn build_submission(vpd: &VPDef) -> Value {
    let descriptor_map: Vec<Value> = vpd
        .input_descriptors
        .iter()
        .enumerate()
        .map(|(i, desc)| {
            serde_json::json!({
                "id": desc.id,
                "format": "jwt_vc_json",
                "path": format!("$.verifiableCredential[{i}]"),
            })
        })
        .collect();

    serde_json::json!({
        "id": format!("urn:uuid:{}", Uuid::new_v4()),
        "definition_id": vpd.id,
        "descriptor_map": descriptor_map,
    })
}
