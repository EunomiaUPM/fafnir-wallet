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

use crate::services::HasRepo;
use async_trait::async_trait;
use chrono::Utc;
use ymir::data::entities::wallet::vc;
use ymir::errors::Outcome;
use ymir::types::issuance::VcBody;
use ymir::types::jwt::Jwt;
use ymir::types::vcs::VcPayload;
use ymir::types::vcs::{VcFormat, VcType};

#[async_trait]
pub trait VcManagerModule: HasRepo + Send + Sync + 'static {
    async fn get_all(&self) -> Outcome<Vec<vc::Model>> {
        self.repo().vcs().get_all(None, None).await
    }

    async fn get_by_id(&self, id: String) -> Outcome<vc::Model> {
        self.repo().vcs().get_by_id(&id).await
    }

    async fn store(&self, plan: vc::Plan) -> Outcome<vc::Model> {
        let doc = match &plan.vc_body {
            VcBody::Jwt(jwt) => {
                let claims = Jwt::<VcPayload>::parse(jwt)?;
                claims.unverified_claims().vc_doc().clone()
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

        self.repo().vcs().create(model).await
    }

    async fn delete(&self, id: String) -> Outcome<()> {
        self.repo().vcs().delete(&id).await
    }
}
