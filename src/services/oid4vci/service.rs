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
use tracing::{debug, info};
use uuid::Uuid;
use ymir::capabilities::{Did, Signer};
use ymir::errors::{BadFormat, Errors, Outcome};
use ymir::services::client::ClientTrait;
use ymir::types::http::Body;
use ymir::types::issuing::{
    AuthServerMetadata, CredReqProof, CredentialDefinition, CredentialRequest, DidPossession,
    GiveVC, IssuerMetadata, IssuingToken, VCCredOffer,
};
use ymir::types::keys::Key;
use ymir::types::wallet::fafnir::{DidEntry, KeyEntry, SigningCtx, VcBodyType, VcEntryReq};
use ymir::utils::{get_query_param, json_headers, ParseHeaderExt, ResponseExt};

use crate::services::oid4vci::Oid4vciTrait;

/// Implementación del flujo OID4VCI desde el lado holder, mimando lo
/// que walt-id expone en `/wallet-api/wallet/{id}/exchange/...`.
pub struct Oid4vciService {
    client: Arc<dyn ClientTrait>,
}

impl Oid4vciService {
    pub fn new(client: Arc<dyn ClientTrait>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Oid4vciTrait for Oid4vciService {
    async fn resolve_offer(&self, uri: &str) -> Outcome<VCCredOffer> {
        info!("OID4VCI: resolve_offer({})", uri);

        let parsed = Url::parse(uri).map_err(|e| {
            Errors::parse(
                format!("Invalid credential offer URI: {uri}"),
                Some(Box::new(e)),
            )
        })?;

        // 1) Variante by-value: `credential_offer=<json url-encoded>`.
        if let Some(json) = optional_query(&parsed, "credential_offer") {
            debug!("OID4VCI: offer by value");
            return serde_json::from_str(&json).map_err(|e| {
                Errors::parse(
                    "credential_offer payload is not valid JSON",
                    Some(Box::new(e)),
                )
            });
        }

        // 2) Variante by-reference: `credential_offer_uri=<url>`.
        if let Some(offer_uri) = optional_query(&parsed, "credential_offer_uri") {
            debug!("OID4VCI: offer by reference → {}", offer_uri);
            let resp = self.client.get(&offer_uri, Some(json_headers())).await?;
            return resp.parse_json().await;
        }

        Err(Errors::format(
            BadFormat::Received,
            "URI lacks both credential_offer and credential_offer_uri",
            None,
        ))
    }

    async fn accept_offer(
        &self,
        offer: &VCCredOffer,
        holder_did: &DidEntry,
        holder_key: KeyEntry,
    ) -> Outcome<VcEntryReq> {
        info!(
            "OID4VCI: accept_offer(issuer={}, holder={})",
            offer.credential_issuer, holder_did.did
        );

        // 1) Issuer metadata.
        let issuer_meta_url = format!(
            "{}/.well-known/openid-credential-issuer",
            offer.credential_issuer
        );
        let issuer_meta: IssuerMetadata = self
            .client
            .get(&issuer_meta_url, Some(json_headers()))
            .await?
            .parse_json()
            .await?;
        debug!("Issuer metadata loaded: {}", issuer_meta.credential_endpoint);

        // 2) OAuth authorization-server metadata → token_endpoint.
        let auth_base = issuer_meta
            .authorization_servers
            .first()
            .cloned()
            .unwrap_or_else(|| offer.credential_issuer.clone());
        let auth_meta_url = format!("{}/.well-known/oauth-authorization-server", auth_base);
        let auth_meta: AuthServerMetadata = self
            .client
            .get(&auth_meta_url, Some(json_headers()))
            .await?
            .parse_json()
            .await?;
        debug!("Token endpoint: {}", auth_meta.token_endpoint);

        // 3) Pedir el access_token con el pre-authorized_code grant.
        let mut form = HashMap::new();
        form.insert(
            "grant_type".to_string(),
            "urn:ietf:params:oauth:grant-type:pre-authorized_code".to_string(),
        );
        form.insert(
            "pre-authorized_code".to_string(),
            offer
                .grants
                .urn_pre_authorized_code
                .pre_authorized_code
                .clone(),
        );

        let token: IssuingToken = self
            .client
            .post(&auth_meta.token_endpoint, None, Body::Form(form))
            .await?
            .parse_json()
            .await?;
        debug!("Access token recibido (type={})", token.token_type);

        // 4) Construir y firmar el proof JWT (DidPossession enveloped).
        let key: Key = holder_key.try_into()?;
        let did = Did::parse_from_kid(&holder_did.did)?;
        let sig_ctx = SigningCtx::new(did, key);

        let now = Utc::now().timestamp() as u64;
        let exp = now + 300;
        let possession = DidPossession {
            iss: holder_did.did.clone(),
            sub: holder_did.did.clone(),
            aud: offer.credential_issuer.clone(),
            jti: format!("urn:uuid:{}", Uuid::new_v4()),
            iat: now,
            exp,
        };
        let proof_jwt = Signer::sign_enveloped(
            "openid4vci-proof+jwt",
            "json",
            &serde_json::to_value(&possession)?,
            &sig_ctx,
        )?;

        // 5) Construir la CredentialRequest. Tomamos el primer
        // credential_configuration_id del offer y miramos la definición
        // que el issuer publica para conocer el `type`.
        let conf_id = offer
            .credential_configuration_ids
            .first()
            .ok_or_else(|| {
                Errors::format(
                    BadFormat::Received,
                    "credential offer has no credential_configuration_ids",
                    None,
                )
            })?;
        let conf = issuer_meta
            .credential_configurations_supported
            .get(conf_id)
            .ok_or_else(|| {
                Errors::format(
                    BadFormat::Received,
                    format!("issuer does not advertise configuration '{}'", conf_id),
                    None,
                )
            })?;

        let cred_req = CredentialRequest {
            format: conf.format.clone(),
            proof: CredReqProof {
                proof_type: "jwt".to_string(),
                jwt: proof_jwt.as_str().to_string(),
            },
            credential_definition: CredentialDefinition {
                r#type: conf.credential_definition.r#type.clone(),
            },
        };

        // 6) POST al credential endpoint con Bearer.
        let mut headers = json_headers();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token.access_token).parse_header()?,
        );
        let give_vc: GiveVC = self
            .client
            .post(
                &issuer_meta.credential_endpoint,
                Some(headers),
                Body::json(&cred_req)?,
            )
            .await?
            .parse_json()
            .await?;
        info!("VC recibida (format={})", give_vc.format);

        // 7) Devolver entrada lista para persistir. Como id estable
        // usamos un UUID nuevo — basta para que no choque con otras VCs
        // en el repo on-disk.
        Ok(VcEntryReq {
            id: Uuid::new_v4().to_string(),
            r#type: VcBodyType::Jwt(give_vc.credential),
        })
    }
}

/// `get_query_param` falla si el param no existe; aquí lo queremos
/// convertir en `Option` para poder probar dos alternativas en orden.
fn optional_query(url: &Url, key: &str) -> Option<String> {
    get_query_param(url, key).ok()
}
