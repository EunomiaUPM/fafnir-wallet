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
use axum::http::HeaderMap;
use axum::http::header::{ACCEPT, CONTENT_TYPE};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};
use url::Url;
use uuid::Uuid;
use ymir::capabilities::Signer;
use ymir::data::entities::wallet::vc;
use ymir::errors::{BadFormat, Errors, Outcome};
use ymir::services::client::ClientTrait;
use ymir::services::vault::{VaultService, VaultTrait};
use ymir::types::dids::Did;
use ymir::types::http::HttpBody;
use ymir::types::issuance::VcBody;
use ymir::types::jwt::VPJwtClaims;
use ymir::types::keys::{PrivateKey, SigningCtx};
use ymir::types::secrets::PemHelper;
use ymir::types::vcs::{VPDef, W3cDataModelVersion};
use ymir::types::vps::VpDocument;
use ymir::types::wallet::KeyRef;
use ymir::utils::{ParseHeaderExt, ResponseExt, get_query_param, http_client, json_headers};

use crate::services::oid4vp::Oid4vpTrait;
use crate::utils::{normalize_host, parse_or_fail};

pub struct Oid4vpService {
    vault: Arc<VaultService>,
    is_docker: bool,
}

impl Oid4vpService {
    pub fn new(vault: Arc<VaultService>, is_docker: bool) -> Self {
        Self { vault, is_docker }
    }
}

#[async_trait]
impl Oid4vpTrait for Oid4vpService {
    async fn resolve_request(&self, uri: &str) -> Outcome<VPDef> {
        let uri = normalize_host(uri, self.is_docker);
        let parsed = Url::parse(&uri)
            .map_err(|e| Errors::parse(format!("Invalid offer URI: {uri}"), Some(Box::new(e))))?;

        if let Ok(json) = get_query_param(&parsed, "presentation_definition") {
            debug!("OID4VCI: offer by value");
            return serde_json::from_str(&json).map_err(|e| {
                Errors::parse(
                    "credential_offer payload is not valid JSON",
                    Some(Box::new(e)),
                )
            });
        }

        if let Ok(uri) = get_query_param(&parsed, "presentation_definition_uri") {
            debug!("OID4VCI: offer by reference → {}", uri);
            let uri = normalize_host(uri, self.is_docker);
            let res = http_client().get(&uri, Some(json_headers())).await?;
            return parse_or_fail(res, &uri, "GET").await;
        }

        Err(Errors::format(
            BadFormat::Received,
            "URI lacks both credential_offer and credential_offer_uri",
            None,
        ))
    }

    async fn present(
        &self,
        uri: &str,
        vpd: &VPDef,
        selected_vcs: &[vc::Model],
        holder_did: &Did,
        holder_key: &KeyRef,
    ) -> Outcome<Option<String>> {
        let parsed = Url::parse(&uri)
            .map_err(|e| Errors::parse(format!("Invalid request URI: {uri}"), Some(Box::new(e))))?;

        let client_id = get_query_param(&parsed, "client_id")?;
        let nonce = get_query_param(&parsed, "nonce")?;
        let response_uri = get_query_param(&parsed, "response_uri")?;
        let response_uri = normalize_host(response_uri, self.is_docker);
        // `state` is OPTIONAL per OID4VP Draft 20: if the verifier included it,
        // we must echo it back in the response; otherwise we omit it.
        let state: Option<String> = parsed
            .query_pairs()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.into_owned());

        let vc_jwts: Vec<String> = selected_vcs
            .iter()
            .map(|vc| match &vc.vc_body {
                VcBody::Jwt(s) => Ok(s.clone()),
                VcBody::JsonLd(_) => Err(Errors::not_impl(
                    "Embedding a raw JSON VC in a JWT VP is not supported in the base impl",
                    None,
                )),
            })
            .collect::<Outcome<Vec<_>>>()?;

        let vp_doc = VpDocument {
            context: vec![W3cDataModelVersion::default().context().to_string()],
            id: vpd.id.clone(),
            r#type: vec!["VerifiablePresentation".to_string()],
            holder: Some(holder_did.id().to_string()),
            verifiable_credential: vc_jwts,
        };

        let now = Utc::now().timestamp();
        let claims = VPJwtClaims {
            aud: client_id.clone(),
            nonce: nonce.clone(),
            iss: Some(holder_did.id().to_string()),
            sub: Some(holder_did.id().to_string()),
            jti: Some(format!("urn:uuid:{}", Uuid::new_v4())),
            nbf: Some(now),
            exp: Some(now + 300),
            iat: Some(now),
            vp: vp_doc,
        };

        // 3. Firma criptográfica con la clave recuperada del Vault
        let key: PemHelper = self.vault.read(None, holder_key.internal()).await?;
        let private_key = PrivateKey::from_safe_pem(key.pem(), key.kty(), key.crv())?;

        let sig_ctx = SigningCtx::new(
            holder_did.clone(),
            private_key,
            holder_key.fragment().to_string(),
        );

        let vp_jwt =
            Signer::sign_enveloped(&sig_ctx, "vp+jwt", "json", &serde_json::to_value(&claims)?)?;

        let submission = build_submission(vpd);

        let mut form = HashMap::new();
        form.insert("vp_token".to_string(), vp_jwt.as_str().to_string());
        form.insert(
            "presentation_submission".to_string(),
            serde_json::to_string(&submission)?,
        );
        if let Some(state) = state {
            form.insert("state".to_string(), state);
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse_header()?,
        );
        headers.insert(ACCEPT, "application/json".parse_header()?);

        let resp = http_client()
            .post(&response_uri, Some(headers), HttpBody::form(&form)?)
            .await?;

        if !resp.status().is_success() {
            return Err(Errors::forbidden("Verification failed", None))
        }
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
