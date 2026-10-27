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
use tracing::{debug, warn};
use uuid::Uuid;
use ymir::capabilities::{Signer, Verifier};
use ymir::data::entities::wallet::vc;
use ymir::errors::{ErrorReason, Errors, Outcome};
use ymir::services::client::ClientTrait;
use ymir::services::vault::{VaultService, VaultTrait};
use ymir::types::dids::Did;
use ymir::types::http::HttpBody;
use ymir::types::jwt::{Jwt, JwtHeaderTrait};
use ymir::types::jwt::JwtClaims;
use ymir::types::vps::{VpJwtClaimsBuilder, VpPayload};
use ymir::types::keys::{PrivateKey, SigningCtx};
use ymir::types::secrets::PemHelper;
use ymir::types::verification::request_object::{
    REQUEST_OBJECT_MEDIA_TYPE, SELF_ISSUED_AUDIENCE,
};
use ymir::types::verification::{DirectPostResponse, RequestObjectPayload};
use ymir::types::wallet::KeyRef;
use ymir::utils::{ParseHeaderExt, ResponseExt, http_client};

use crate::services::oid4vp::{presented_vcs, Oid4vpTrait};
use ymir::types::verification::{
    AuthorizationRequest, PresentationRequest, RequestReference, UNTRUSTED_REQUEST, VerifierProof,
};
use crate::utils::normalize_host;

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
        AuthorizationRequest::is_1_0(uri)
    }

    async fn resolve_request(&self, uri: &str) -> Outcome<PresentationRequest> {
        let Some(reference) = RequestReference::parse(uri)? else {
            return Ok(AuthorizationRequest::parse(uri)?.into());
        };
        let did = reference.did().to_string();
        let mut request: PresentationRequest = self.fetch_request_object(&reference).await?.into();
        request.verifier = VerifierProof::SignedBy { did };
        Ok(request)
    }

    async fn present(
        &self,
        request: &PresentationRequest,
        selected_vcs: &[(String, Vec<vc::Model>)],
        holder_did: &Did,
        holder_key: &KeyRef,
    ) -> Outcome<Option<String>> {
        let key: PemHelper = self.vault.read(None, holder_key.internal()).await?;
        let private_key = PrivateKey::from_safe_pem(key.pem(), key.kty(), key.crv())?;
        let sig_ctx = SigningCtx::new(
            holder_did.clone(),
            private_key,
            holder_key.fragment().to_string(),
        );

        // §8.1: query id -> array of presentations answering it.
        let mut vp_token: HashMap<&str, Vec<String>> = HashMap::new();
        for (query_id, vcs) in selected_vcs {
            let claims = presentation_claims(request, vcs, holder_did)?;
            let vp_jwt = Signer::sign_enveloped(&sig_ctx, &claims)?;
            vp_token.insert(query_id, vec![vp_jwt.as_str().to_string()]);
        }

        let mut form = HashMap::new();
        form.insert("vp_token".to_string(), serde_json::to_string(&vp_token)?);
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

        // §8.2: a JSON object, maybe with where to send the user next.
        let body = resp.parse_text().await.unwrap_or_default();
        if body.trim().is_empty() {
            return Ok(None);
        }
        match serde_json::from_str::<DirectPostResponse>(&body) {
            Ok(answer) => Ok(answer.redirect_uri),
            Err(_) => {
                warn!("Verifier response was not a JSON object: {}", body);
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

impl Oid4vpService {
    /// Fetches the Request Object of a request by reference (§5.10) and checks it: `typ`,
    /// signed by the DID of the Client Identifier for any wallet (`aud`), not expired, and
    /// echoing the `wallet_nonce` sent with a POST.
    async fn fetch_request_object(
        &self,
        reference: &RequestReference,
    ) -> Outcome<AuthorizationRequest> {
        let request_uri = normalize_host(&reference.request_uri, self.is_docker);
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, REQUEST_OBJECT_MEDIA_TYPE.parse_header()?);

        let wallet_nonce = reference.post.then(|| Uuid::new_v4().simple().to_string());
        let resp = match &wallet_nonce {
            Some(nonce) => {
                let mut form = HashMap::new();
                form.insert("wallet_nonce".to_string(), nonce.clone());
                let headers = form_headers_accepting(REQUEST_OBJECT_MEDIA_TYPE)?;
                http_client()
                    .post(&request_uri, Some(headers), HttpBody::form(&form)?)
                    .await?
            }
            None => http_client().get(&request_uri, Some(headers)).await?,
        };
        if !resp.status().is_success() {
            return Err(Errors::forbidden(
                format!("The verifier did not serve its Request Object: {}", resp.status()),
                None,
            ));
        }
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        if !content_type.starts_with(REQUEST_OBJECT_MEDIA_TYPE)
            && !content_type.starts_with("application/jwt")
        {
            warn!("Request Object served as '{content_type}'");
        }

        let request = Jwt::<RequestObjectPayload>::parse(resp.parse_text().await?.trim())?;
        // `verify_enveloped` checks the `typ` (oauth-authz-req+jwt) and the `aud`.
        let request = Verifier::verify_enveloped(request, Some(SELF_ISSUED_AUDIENCE.to_string()))
            .await
            .map_err(|e| untrusted(format!("Invalid Request Object signature: {}", e.reason())))?;
        let kid = request.kid().map_err(|e| untrusted(e.reason().to_string()))?;
        if kid.did().id() != reference.did() {
            return Err(untrusted("The Request Object is not signed by the DID of its client_id"));
        }
        match request.claims().exp() {
            Some(exp) if exp >= Utc::now().timestamp() => {}
            _ => return Err(untrusted("The Request Object has expired")),
        }
        if wallet_nonce.is_some() && request.body().wallet_nonce != wallet_nonce {
            return Err(untrusted("The Request Object does not echo the wallet_nonce"));
        }

        debug!("Request Object of {} verified", reference.client_id);
        AuthorizationRequest::from_request_object(&request, reference)
    }
}

/// The claims of one presentation: the credentials, bound to the request by `aud` (the full
/// Client Identifier, prefix included) and `nonce` (B.1.3.1.2).
fn untrusted(message: impl Into<String>) -> Errors {
    Errors::validation_with(vec![ErrorReason {
        path: "request_object".to_string(),
        code: UNTRUSTED_REQUEST,
        message: message.into(),
    }])
}

fn presentation_claims(
    request: &PresentationRequest,
    vcs: &[vc::Model],
    holder_did: &Did,
) -> Outcome<JwtClaims<VpPayload>> {
    let (vc_jwts, version) = presented_vcs(vcs)?;
    let now = Utc::now();
    Ok(VpJwtClaimsBuilder::new(version)
        .presentation(format!("urn:uuid:{}", Uuid::new_v4()), holder_did.id(), vc_jwts)
        .aud(request.client_id.clone())
        .nonce(request.nonce.clone())
        .iat(now)
        .exp(now + Duration::seconds(300))
        .build())
}

/// §8.2: the answer goes as a form; the verifier replies JSON.
fn form_headers() -> Outcome<HeaderMap> {
    form_headers_accepting("application/json")
}

/// A form post accepting `accept` back.
fn form_headers_accepting(accept: &str) -> Outcome<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        "application/x-www-form-urlencoded".parse_header()?,
    );
    headers.insert(ACCEPT, accept.parse_header()?);
    Ok(headers)
}
