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

use crate::modules::KeyManagerModule;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ymir::data::entities::wallet::key;
use ymir::errors::AppResult;
use ymir::utils::extract_payload;

pub struct KeyRouter {
    core: Arc<dyn KeyManagerModule>,
}

impl KeyRouter {
    pub fn new(core: Arc<dyn KeyManagerModule>) -> Self {
        Self { core }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/all", get(Self::get_all))
            .route("/new", post(Self::create))
            .route("/{id}", get(Self::get_by_id).delete(Self::delete))
            .with_state(self.core)
    }

    async fn get_all(
        State(core): State<Arc<dyn KeyManagerModule>>,
    ) -> AppResult<Json<Vec<key::Model>>> {
        Ok(Json(core.get_all().await?))
    }

    async fn create(
        State(core): State<Arc<dyn KeyManagerModule>>,
        payload: Result<Json<key::Plan>, JsonRejection>,
    ) -> AppResult<(StatusCode, Json<key::Model>)> {
        let payload = extract_payload(payload)?;
        Ok((StatusCode::CREATED, Json(core.create(payload).await?)))
    }

    async fn get_by_id(
        State(core): State<Arc<dyn KeyManagerModule>>,
        Path(id): Path<String>,
    ) -> AppResult<Json<key::Model>> {
        Ok(Json(core.get_by_id(id).await?))
    }

    async fn delete(
        State(core): State<Arc<dyn KeyManagerModule>>,
        Path(id): Path<String>,
    ) -> AppResult<StatusCode> {
        core.delete(id).await?;
        Ok(StatusCode::OK)
    }
}
