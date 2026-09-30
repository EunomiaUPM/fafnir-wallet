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

use crate::services::{HasOid4Vci, HasOid4Vp, HasRepo};
use async_trait::async_trait;
use chrono::Utc;
use ymir::data::entities::wallet::vc;
use ymir::errors::{Errors, Outcome};
use ymir::types::dids::Did;
use ymir::types::issuance::VcBody;
use ymir::types::issuance::VcCredOffer;
use ymir::types::jwt::{Jwt, VCJwtClaims};
use ymir::types::vcs::VPDef;
use ymir::types::vcs::{VcFormat, VcType};

#[async_trait]
pub trait Oid4VcModule: HasRepo + HasOid4Vp + HasOid4Vci + Send + Sync + 'static {
    // ===== OID4VCI ===============================================================================

    async fn process_oid4vci(&self, uri: &str) -> Outcome<()> {
        let offer = self.oid4vci().resolve_offer(uri).await?;
        self.vci_accept_offer(offer).await
    }

    async fn vci_resolve_offer(&self, uri: &str) -> Outcome<VcCredOffer> {
        self.oid4vci().resolve_offer(uri).await
    }

    async fn vci_accept_offer(&self, offer: VcCredOffer) -> Outcome<()> {
        let plans = self.oid4vci().accept_offer(&offer).await?;
        for plan in plans {
            let doc = match &plan.vc_body {
                VcBody::Jwt(jwt) => {
                    let claims: VCJwtClaims = Jwt::parse(jwt)?.unsafe_claims()?;
                    claims.vc_doc().clone()
                }
                VcBody::JsonLd(value) => serde_json::from_value(value.clone())?,
            };

            let vc_format = match &plan.vc_body {
                VcBody::Jwt(_) => VcFormat::JwtVcJson,
                VcBody::JsonLd(_) => VcFormat::LdpVc,
            };

            let model = vc::Model {
                id: doc.id().to_string(),
                vc_body: plan.vc_body,
                vc_type: doc
                    .specialized_type()
                    .unwrap_or(VcType::Other("Unknown".to_string())),
                vc_format,
                holder_did: doc.holder_did().unwrap_or("Unknown").to_string(),
                issuer_did: doc.issuer.id().to_string(),
                parsed_document: serde_json::to_value(&doc)?,
                valid_until: doc.valid_until,
                added_on: Utc::now(),
            };

            self.repo().vcs().create(model).await?;
        }
        Ok(())
    }

    // ===== OID4VP ================================================================================
    async fn process_oid4vp(&self, uri: &str) -> Outcome<Option<String>> {
        self.vp_present(uri).await
    }

    async fn vp_resolve_request(&self, uri: &str) -> Outcome<VPDef> {
        self.oid4vp().resolve_request(uri).await
    }

    async fn vp_present(&self, uri: &str) -> Outcome<Option<String>> {
        let vpd = self.oid4vp().resolve_request(uri).await?;
        let mut selected_vcs = Vec::new();
        let mut holder_did = None;
        let mut default_key = None;

        for desc in &vpd.input_descriptors {
            let vcs = self.repo().vcs().filter_by_desc(desc).await?;
            let now = Utc::now();
            let valid: Vec<vc::Model> = vcs
                .into_iter()
                .filter(|vc| vc.valid_until.map_or(true, |exp| now < exp))
                .collect();

            let vc = valid.first().ok_or_else(|| {
                Errors::missing_resource("vc", "Missing vc that matches input descriptor", None)
            })?;

            if holder_did.is_none() {
                let did = Did::parse(&vc.holder_did)?;
                let did_model = self.repo().dids().get_by_did(did.id()).await?;
                holder_did = Some(did);
                default_key = Some(did_model.default_key);
            }

            selected_vcs.push(vc.clone());
        }

        let did = holder_did
            .ok_or_else(|| Errors::missing_resource("holder_did", "No holder DID found", None))?;
        let key_ref = default_key
            .ok_or_else(|| Errors::missing_resource("default_key", "No default key found", None))?;

        self.oid4vp()
            .present(uri, &vpd, &selected_vcs, &did, &key_ref)
            .await
    }
}
