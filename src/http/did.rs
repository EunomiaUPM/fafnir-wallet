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

use crate::core::traits::CoreDidsTrait;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ymir::errors::AppResult;
use ymir::types::wallet::fafnir::{DidEntry, DidEntryReq};

pub struct DidRouter {
    core: Arc<dyn CoreDidsTrait>,
}

impl DidRouter {
    pub fn new(core: Arc<dyn CoreDidsTrait>) -> Self {
        Self { core }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/all", get(Self::get_all))
            .route("/new", post(Self::create))
            .route("/default", get(Self::get_default))
            .route("/{id}", get(Self::get_by_id).delete(Self::delete))
            .route("/{id}/default", post(Self::set_default))
            .with_state(self.core)
    }

    async fn get_all(State(core): State<Arc<dyn CoreDidsTrait>>) -> AppResult<Json<Vec<DidEntry>>> {
        Ok(Json(core.get_all().await?))
    }

    async fn get_by_id(
        State(core): State<Arc<dyn CoreDidsTrait>>,
        Path(id): Path<String>,
    ) -> AppResult<Json<DidEntry>> {
        Ok(Json(core.get_by_id(id).await?))
    }

    async fn get_default(State(core): State<Arc<dyn CoreDidsTrait>>) -> AppResult<Json<DidEntry>> {
        Ok(Json(core.get_default().await?))
    }

    async fn create(
        State(core): State<Arc<dyn CoreDidsTrait>>,
        Json(entry): Json<DidEntryReq>,
    ) -> AppResult<Json<DidEntry>> {
        Ok(Json(core.create(entry).await?))
    }

    async fn delete(
        State(core): State<Arc<dyn CoreDidsTrait>>,
        Path(id): Path<String>,
    ) -> AppResult<StatusCode> {
        core.delete(id).await?;
        Ok(StatusCode::NO_CONTENT)
    }

    async fn set_default(
        State(core): State<Arc<dyn CoreDidsTrait>>,
        Path(id): Path<String>,
    ) -> AppResult<StatusCode> {
        core.set_default(id).await?;
        Ok(StatusCode::OK)
    }
}
