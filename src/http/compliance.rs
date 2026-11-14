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
use axum::routing::post;
use axum::{Json, Router};
use ymir::data::entities::wallet::vc;
use ymir::errors::AppResult;
use ymir::types::wallet::ComplianceRequest;

use crate::modules::ComplianceModule;

/// Gaia-X compliance by plain POST (`/compliance/request`).
pub struct ComplianceRouter {
    core: Arc<dyn ComplianceModule>,
}

impl ComplianceRouter {
    pub fn new(core: Arc<dyn ComplianceModule>) -> Self {
        Self { core }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/request", post(Self::request))
            .with_state(self.core)
    }

    /// POST /compliance/request  body: `{ "url": "...", "vcid": "...", "types": [...] }`.
    /// Answers 201 with the credential stored.
    async fn request(
        State(core): State<Arc<dyn ComplianceModule>>,
        Json(request): Json<ComplianceRequest>,
    ) -> AppResult<(StatusCode, Json<vc::Model>)> {
        Ok((StatusCode::CREATED, Json(core.request_compliance(&request).await?)))
    }
}
