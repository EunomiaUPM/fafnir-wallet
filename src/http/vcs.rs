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

use crate::core::traits::CoreVcsTrait;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use ymir::errors::AppResult;
use ymir::types::wallet::fafnir::VcEntry;

pub struct VcsRouter {
    core: Arc<dyn CoreVcsTrait>,
}

impl VcsRouter {
    pub fn new(core: Arc<dyn CoreVcsTrait>) -> Self {
        Self { core }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/all", get(Self::list))
            .route("/{id}", get(Self::get_by_id).delete(Self::delete))
            .with_state(self.core)
    }

    async fn list(State(core): State<Arc<dyn CoreVcsTrait>>) -> AppResult<Json<Vec<VcEntry>>> {
        Ok(Json(core.get_all().await?))
    }

    async fn get_by_id(
        State(core): State<Arc<dyn CoreVcsTrait>>,
        Path(id): Path<String>,
    ) -> AppResult<Json<VcEntry>> {
        Ok(Json(core.get_by_id(id).await?))
    }

    async fn delete(
        State(core): State<Arc<dyn CoreVcsTrait>>,
        Path(id): Path<String>,
    ) -> AppResult<StatusCode> {
        core.delete(id).await?;
        Ok(StatusCode::NO_CONTENT)
    }
}
