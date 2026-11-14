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
use axum::http::HeaderMap;
use axum::http::header::{ACCEPT, CONTENT_TYPE};
use chrono::{Duration, Utc};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;
use ymir::capabilities::{Signer, Verifier};
use ymir::data::entities::wallet::vc;
use ymir::errors::{BadFormat, Errors, Outcome};
use ymir::services::HasVault;
use ymir::services::client::ClientTrait;
use ymir::services::vault::VaultTrait;
use ymir::types::http::HttpBody;
use ymir::types::issuance::VcBody;
use ymir::types::jwt::Jwt;
use ymir::types::keys::{PrivateKey, SigningCtx};
use ymir::types::secrets::PemHelper;
use ymir::types::vcs::VcPayload;
use ymir::types::verification::{DcqlQuery, Requested};
use ymir::types::vps::VpJwtClaimsBuilder;
use ymir::types::wallet::ComplianceRequest;
use ymir::utils::{ParseHeaderExt, ResponseExt, protocol_client};

use crate::modules::{Oid4VcModule, VcManagerModule};
use crate::services::oid4vp::presented_vcs;
use crate::utils::normalize_host;

/// Gaia-X compliance by plain POST: one presentation with every credential, and the credential
/// the service answers, stored (flow of the Loire compliance API and of Danube extensions).
#[async_trait]
pub trait ComplianceModule: Oid4VcModule + VcManagerModule + HasVault + Send + Sync + 'static {
    /// Whether the wallet runs in Docker, where `localhost` is reached as `host.docker.internal`.
    fn is_docker(&self) -> bool;

    async fn request_compliance(&self, request: &ComplianceRequest) -> Outcome<vc::Model> {
        let query = DcqlQuery::for_types(&request.types);
        let (selected, did, key_ref) = self.vp_select(&Requested::Dcql(query)).await?;
        let vcs: Vec<vc::Model> = selected.into_iter().flat_map(|(_, vcs)| vcs).collect();
        let (jwts, version) = presented_vcs(&vcs)?;

        let key: PemHelper = self.vault().read(None, key_ref.internal()).await?;
        let private_key = PrivateKey::from_safe_pem(key.pem(), key.kty(), key.crv())?;
        let sig_ctx = SigningCtx::new(did.clone(), private_key, key_ref.fragment().to_string());
        let now = Utc::now();
        let claims = VpJwtClaimsBuilder::new(version)
            .presentation(format!("urn:uuid:{}", Uuid::new_v4()), did.id(), jwts)
            .unbound()
            .iat(now)
            .exp(now + Duration::seconds(300))
            .build();
        let vp = Signer::sign_enveloped(&sig_ctx, &claims)?;

        let vcid = request
            .vcid
            .clone()
            .unwrap_or_else(|| format!("urn:uuid:{}", Uuid::new_v4()));
        let target = normalize_host(&request.url, self.is_docker());
        let mut url = url::Url::parse(&target).map_err(|e| {
            Errors::format(BadFormat::Received, "Invalid compliance service URL", Some(Box::new(e)))
        })?;
        url.query_pairs_mut().append_pair("vcid", &vcid);
        let url = url.to_string();
        info!("Requesting Gaia-X compliance at {url}");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/vp+jwt".parse_header()?);
        headers.insert(ACCEPT, "application/vc+jwt".parse_header()?);
        // The service resolves DIDs and chains and checks shapes first: Loire takes ~20 s.
        let res = protocol_client()
            .post(&url, Some(headers), HttpBody::Raw(vp.as_str().to_string()))
            .await?;
        let status = res.status();
        let body = res.parse_text().await?;
        if !status.is_success() {
            return Err(Errors::forbidden(
                format!("Compliance refused: {}", refusal_reason(&body, status.as_u16())),
                None,
            ));
        }

        let issued = Jwt::<VcPayload>::parse(body.trim())?;
        Verifier::verify_enveloped(issued, None).await?;
        self.store(vc::Plan {
            vc_body: VcBody::Jwt(body.trim().to_string()),
        })
        .await
    }
}

/// What a compliance service says when it refuses, or its status. `details` first: ymir puts the
/// reason there and a generic text in `message`; NestJS (Loire) puts it in `message`.
fn refusal_reason(body: &str, status: u16) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|body| {
            ["details", "error_description", "message", "error"]
                .into_iter()
                .find_map(|key| match body.get(key)? {
                    Value::String(text) => Some(text.clone()),
                    other => Some(other.to_string()),
                })
        })
        .unwrap_or_else(|| format!("the service answered {status}"))
}
