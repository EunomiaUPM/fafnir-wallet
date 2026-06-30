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
use axum::http::HeaderMap;
use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use url::Url;
use ymir::capabilities::Signer;
use ymir::data::entities::wallet::vc;
use ymir::errors::{BadFormat, Errors, MissingAction, Outcome};
use ymir::services::client::ClientTrait;
use ymir::services::vault::{VaultService, VaultTrait};
use ymir::types::http::HttpBody;
use ymir::types::issuance::{
    AuthServerMetadata, CredReqProof, CredentialRequest, DidPossession, GiveVC, IssuerMetadata,
    IssuingToken, OidcGrantType, PreAuthorizedCodeGrant, TokenRequest, VcCredOffer,
};
use ymir::types::keys::{PrivateKey, SigningCtx};
use ymir::types::secrets::PemHelper;
use ymir::types::vcs::VcTypeConfig;
use ymir::types::wallet::Identity;
use ymir::utils::{ParseHeaderExt, get_query_param, http_client, json_headers};

use crate::services::oid4vci::Oid4vciTrait;
use crate::utils::{normalize_host, parse_or_fail};

pub struct Oid4vciService {
    identity: Arc<RwLock<Option<Identity>>>,
    vault: Arc<VaultService>,
    is_docker: bool,
}

impl Oid4vciService {
    pub fn new(
        identity: Arc<RwLock<Option<Identity>>>,
        vault: Arc<VaultService>,
        is_docker: bool,
    ) -> Self {
        Self {
            identity,
            vault,
            is_docker,
        }
    }
}

#[async_trait]
impl Oid4vciTrait for Oid4vciService {
    async fn resolve_offer(&self, uri: &str) -> Outcome<VcCredOffer> {
        let parsed = Url::parse(uri)
            .map_err(|e| Errors::parse(format!("Invalid offer URI: {uri}"), Some(Box::new(e))))?;

        if let Ok(json) = get_query_param(&parsed, "credential_offer") {
            debug!("OID4VCI: offer by value");
            return serde_json::from_str(&json).map_err(|e| {
                Errors::parse(
                    "credential_offer payload is not valid JSON",
                    Some(Box::new(e)),
                )
            });
        }

        if let Ok(uri) = get_query_param(&parsed, "credential_offer_uri") {
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

    async fn accept_offer(&self, offer: &VcCredOffer) -> Outcome<Vec<vc::Plan>> {
        let pre_auth = self.extract_pre_auth_grant(offer)?;

        let issuer_meta = self.resolve_issuer(offer).await?;
        let auth_meta = self
            .resolve_auth(&issuer_meta, pre_auth.authorization_server.clone())
            .await?;

        let token = self.request_token(&auth_meta, &pre_auth).await?;

        let proof_jwt = self
            .build_proof(&offer.credential_issuer, token.c_nonce.as_deref())
            .await?;

        let mut plans = Vec::with_capacity(offer.credential_configuration_ids.len());
        for conf_id in &offer.credential_configuration_ids {
            if !issuer_meta
                .credential_configurations_supported
                .contains_key(conf_id)
            {
                continue;
            }

            let plan = self
                .request_credential(
                    &issuer_meta.credential_endpoint,
                    conf_id,
                    &proof_jwt,
                    &token.access_token,
                )
                .await?;
            plans.push(plan);
        }

        info!("received VC(s) {} ", plans.len());
        Ok(plans)
    }
}

impl Oid4vciService {
    fn extract_pre_auth_grant(&self, offer: &VcCredOffer) -> Outcome<PreAuthorizedCodeGrant> {
        if offer.credential_configuration_ids.is_empty() {
            return Err(Errors::format(
                BadFormat::Received,
                "credential offer has no credential_configuration_ids",
                None,
            ));
        }

        let grants = offer
            .grants
            .as_ref()
            .ok_or_else(|| Errors::not_impl("An empty grant is not supported", None))?;

        grants.pre_authorized_code.clone().ok_or_else(|| {
            Errors::not_impl(
                "Missing pre-authorized_code grant — only this grant type is supported",
                None,
            )
        })
    }

    async fn resolve_issuer(&self, offer: &VcCredOffer) -> Outcome<IssuerMetadata> {
        let host = normalize_host(&offer.credential_issuer, self.is_docker);
        let url = format!("{}/.well-known/openid-credential-issuer", host);
        let res = http_client().get(&url, Some(json_headers())).await?;
        parse_or_fail(res, &url, "GET").await
    }

    async fn resolve_auth(
        &self,
        issuer_metadata: &IssuerMetadata,
        specific_one: Option<String>,
    ) -> Outcome<AuthServerMetadata> {
        let auth_url = specific_one.unwrap_or_else(|| {
            issuer_metadata
                .authorization_servers
                .as_ref()
                .and_then(|s| s.first().cloned())
                .unwrap_or_else(|| issuer_metadata.credential_issuer.clone())
        });

        let auth_url = normalize_host(&auth_url, self.is_docker);
        let url = format!("{}/.well-known/oauth-authorization-server", auth_url);
        let res = http_client().get(&url, Some(json_headers())).await?;
        parse_or_fail(res, &url, "GET").await
    }

    async fn request_token(
        &self,
        auth_meta: &AuthServerMetadata,
        pre_auth: &PreAuthorizedCodeGrant,
    ) -> Outcome<IssuingToken> {
        let token_req = TokenRequest {
            grant_type: OidcGrantType::PreAuthorizedCode,
            pre_authorized_code: pre_auth.pre_authorized_code.clone(),
            tx_code: None,
            client_id: None,
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse_header()?,
        );
        headers.insert(ACCEPT, "application/json".parse_header()?);

        let token_endpoint = normalize_host(&auth_meta.token_endpoint, self.is_docker);
        let res = http_client()
            .post(
                &token_endpoint,
                Some(headers),
                HttpBody::form(&token_req)?,
            )
            .await?;

        parse_or_fail(res, &token_endpoint, "POST").await
    }

    async fn build_proof(&self, credential_issuer: &str, c_nonce: Option<&str>) -> Outcome<String> {
        let identity = self.identity.read().await;

        let identity = identity.as_ref().ok_or_else(|| {
            Errors::missing_action(
                MissingAction::Did,
                "There is not a default did stored",
                None,
            )
        })?;

        let key: PemHelper = self.vault.read(None, identity.key_ref().internal()).await?;
        let private_key = PrivateKey::from_safe_pem(key.pem(), key.kty(), key.crv())?;

        let sig_ctx = SigningCtx::new(
            identity.did().clone(),
            private_key,
            identity.key_ref().fragment().to_string(),
        );

        let now = Utc::now().timestamp();
        let possession = DidPossession {
            iss: Some(identity.did().id().to_string()),
            aud: credential_issuer.to_string(),
            iat: now,
            nonce: c_nonce
                .ok_or_else(|| {
                    Errors::format(
                        BadFormat::Received,
                        "Token response missing c_nonce — required for proof JWT",
                        None,
                    )
                })?
                .to_string(),
        };

        let jwt = Signer::sign_enveloped(
            &sig_ctx,
            "openid4vci-proof+jwt",
            "json",
            &serde_json::to_value(&possession)?,
        )?;
        Ok(jwt.as_str().to_string())
    }

    async fn request_credential(
        &self,
        credential_endpoint: &str,
        conf_id: &VcTypeConfig,
        proof_jwt: &str,
        access_token: &str,
    ) -> Outcome<vc::Plan> {
        let req = CredentialRequest {
            credential_configuration_id: Some(conf_id.clone()),
            credential_identifier: None,
            proof: Some(CredReqProof::Jwt {
                jwt: proof_jwt.to_string(),
            }),
            proofs: None,
            credential_response_encryption: None,
        };

        let mut headers = json_headers();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", access_token).parse_header()?,
        );
        let credential_endpoint = normalize_host(credential_endpoint, self.is_docker);
        let res = http_client()
            .post(&credential_endpoint, Some(headers), HttpBody::json(&req)?)
            .await?;

        let give_vc: GiveVC = parse_or_fail(res, &credential_endpoint, "POST").await?;

        let body = give_vc
            .credentials
            .and_then(|mut v| v.pop().map(|item| item.credential))
            .ok_or_else(|| {
                Errors::format(
                    BadFormat::Received,
                    "Issuer returned no credentials in response",
                    None,
                )
            })?;

        Ok(vc::Plan { vc_body: body })
    }
}
