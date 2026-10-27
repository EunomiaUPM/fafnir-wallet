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
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};
use uuid::Uuid;
use ymir::capabilities::Signer;
use ymir::data::entities::wallet::vc;
use ymir::errors::{Errors, Outcome};
use ymir::services::client::ClientTrait;
use ymir::services::vault::{VaultService, VaultTrait};
use ymir::types::dids::Did;
use ymir::types::http::HttpBody;
// use ymir::types::issuance::VcBody;
// use ymir::types::jwt::VPJwtClaims;
use ymir::types::keys::{PrivateKey, SigningCtx};
use ymir::types::secrets::PemHelper;
use ymir::types::vps::VpJwtClaimsBuilder;
// use ymir::types::vcs::{VPDef, W3cDataModelVersion};
use ymir::types::vcs::VPDef;
// use ymir::types::vps::VpDocument;
use ymir::types::wallet::KeyRef;
use ymir::utils::{ParseHeaderExt, ResponseExt, http_client, json_headers};

use crate::services::oid4vp::{Oid4vpTrait, presented_vcs};
use crate::utils::{normalize_host, parse_or_fail};
use ymir::types::verification::{
    DefinitionSource, Draft20Request, PresentationRequest, Requested, VerifierProof,
};

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
    fn handles(&self, uri: &str) -> bool {
        Draft20Request::is_draft20(uri)
    }

    async fn resolve_request(&self, uri: &str) -> Outcome<PresentationRequest> {
        let request = Draft20Request::parse(uri)?;
        let definition = match request.definition {
            DefinitionSource::ByValue(vpd) => vpd,
            DefinitionSource::ByReference(pd_uri) => {
                debug!("OID4VP: presentation definition by reference → {}", pd_uri);
                let pd_uri = normalize_host(pd_uri, self.is_docker);
                let res = http_client().get(&pd_uri, Some(json_headers())).await?;
                parse_or_fail(res, &pd_uri, "GET").await?
            }
        };

        Ok(PresentationRequest {
            client_id: request.client_id,
            response_uri: request.response_uri,
            nonce: request.nonce,
            state: request.state,
            requested: Requested::Definition(definition),
            verifier: VerifierProof::Unsigned,
        })
    }

    /// Draft 20: one presentation with every credential, and the `presentation_submission`
    /// that maps each to its input descriptor.
    async fn present(
        &self,
        request: &PresentationRequest,
        selected_vcs: &[(String, Vec<vc::Model>)],
        holder_did: &Did,
        holder_key: &KeyRef,
    ) -> Outcome<Option<String>> {
        let Requested::Definition(vpd) = &request.requested else {
            return Err(Errors::not_impl(
                "A draft 20 presentation answers a Presentation Definition",
                None,
            ));
        };

        let vcs: Vec<vc::Model> = selected_vcs
            .iter()
            .flat_map(|(_, vcs)| vcs.iter().cloned())
            .collect();
        let (vc_jwts, version) = presented_vcs(&vcs)?;
        let now = Utc::now();
        let claims = VpJwtClaimsBuilder::new(version)
            .presentation(vpd.id.clone(), holder_did.id(), vc_jwts)
            .aud(request.client_id.clone())
            .nonce(request.nonce.clone())
            .iat(now)
            .exp(now + Duration::seconds(300))
            .build();

        let key: PemHelper = self.vault.read(None, holder_key.internal()).await?;
        let private_key = PrivateKey::from_safe_pem(key.pem(), key.kty(), key.crv())?;
        let sig_ctx = SigningCtx::new(
            holder_did.clone(),
            private_key,
            holder_key.fragment().to_string(),
        );
        let vp_jwt = Signer::sign_enveloped(&sig_ctx, &claims)?;

        let mut form = HashMap::new();
        form.insert("vp_token".to_string(), vp_jwt.as_str().to_string());
        form.insert(
            "presentation_submission".to_string(),
            serde_json::to_string(&build_submission(vpd, selected_vcs))?,
        );
        if let Some(state) = &request.state {
            form.insert("state".to_string(), state.clone());
        }

        let response_uri = normalize_host(&request.response_uri, self.is_docker);
        let resp = http_client()
            .post(&response_uri, Some(form_headers()?), HttpBody::form(&form)?)
            .await?;
        if !resp.status().is_success() {
            return Err(Errors::forbidden("Verification failed", None));
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

    async fn refuse(
        &self,
        request: &PresentationRequest,
        error: &str,
        description: &str,
    ) -> Outcome<()> {
        let mut form = HashMap::new();
        form.insert("error".to_string(), error.to_string());
        form.insert("error_description".to_string(), description.to_string());
        if let Some(state) = &request.state {
            form.insert("state".to_string(), state.clone());
        }

        // Any answer will do: the verifier closes the request with a failure on its side.
        let response_uri = normalize_host(&request.response_uri, self.is_docker);
        let resp = http_client()
            .post(&response_uri, Some(form_headers()?), HttpBody::form(&form)?)
            .await?;
        debug!("Refusal sent to {}: {}", response_uri, resp.status());
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct RedirectResponse {
    redirect_uri: String,
}

/// The answer goes as a form; the verifier replies JSON.
fn form_headers() -> Outcome<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        "application/x-www-form-urlencoded".parse_header()?,
    );
    headers.insert(ACCEPT, "application/json".parse_header()?);
    Ok(headers)
}

/// Maps each credential of the presentation, in order, to the input descriptor it answers.
fn build_submission(vpd: &VPDef, selected_vcs: &[(String, Vec<vc::Model>)]) -> Value {
    let descriptor_map: Vec<Value> = selected_vcs
        .iter()
        .flat_map(|(descriptor, vcs)| vcs.iter().map(move |_| descriptor))
        .enumerate()
        .map(|(i, descriptor)| {
            serde_json::json!({
                "id": descriptor,
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
