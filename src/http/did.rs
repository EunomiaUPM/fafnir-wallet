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

use crate::modules::DidManagerModule;
use crate::types::DidSearch;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ymir::data::entities::wallet::did;
use ymir::errors::AppResult;
use ymir::types::secrets::StringHelper;
use ymir::utils::extract_payload;

pub struct DidRouter {
    core: Arc<dyn DidManagerModule>,
}

impl DidRouter {
    pub fn new(core: Arc<dyn DidManagerModule>) -> Self {
        Self { core }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/all", get(Self::get_all))
            .route("/new", post(Self::create))
            .route("/default", get(Self::get_default))
            .route("/default/did", post(Self::set_default_by_did))
            .route("/default/{id}", post(Self::set_default_by_id))
            .route(
                "/did/key/{key_id}",
                post(Self::add_key_by_did).delete(Self::delete_key_by_did),
            )
            .route("/did/key/default/{key_id}", post(Self::default_key_by_did))
            .route("/did", post(Self::get_by_did).delete(Self::delete_by_did))
            .route(
                "/{id}/key/{key_id}",
                post(Self::add_key_by_id).delete(Self::delete_key_by_id),
            )
            .route("/{id}/key/default/{key_id}", post(Self::default_key_by_id))
            .route("/{id}", get(Self::get_by_id).delete(Self::delete_by_id))
            .with_state(self.core)
    }

    async fn get_all(
        State(core): State<Arc<dyn DidManagerModule>>,
    ) -> AppResult<Json<Vec<did::Model>>> {
        Ok(Json(core.get_all().await?))
    }

    async fn create(
        State(core): State<Arc<dyn DidManagerModule>>,
        payload: Result<Json<did::Plan>, JsonRejection>,
    ) -> AppResult<(StatusCode, Json<did::Model>)> {
        let payload = extract_payload(payload)?;
        Ok((StatusCode::CREATED, Json(core.create(payload).await?)))
    }

    async fn get_default(
        State(core): State<Arc<dyn DidManagerModule>>,
    ) -> AppResult<Json<did::Model>> {
        Ok(Json(core.get_default().await?))
    }

    async fn set_default_by_did(
        State(core): State<Arc<dyn DidManagerModule>>,
        payload: Result<Json<StringHelper>, JsonRejection>,
    ) -> AppResult<Json<did::Model>> {
        let payload = extract_payload(payload)?;
        Ok(Json(
            core.set_default(DidSearch::Did(payload.data().to_string()))
                .await?,
        ))
    }

    async fn set_default_by_id(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path(id): Path<String>,
    ) -> AppResult<Json<did::Model>> {
        Ok(Json(core.set_default(DidSearch::Id(id)).await?))
    }

    async fn add_key_by_did(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path(key_id): Path<String>,
        payload: Result<Json<StringHelper>, JsonRejection>,
    ) -> AppResult<Json<did::Model>> {
        let payload = extract_payload(payload)?;
        Ok(Json(
            core.add_key(DidSearch::Did(payload.data().to_string()), key_id)
                .await?,
        ))
    }

    async fn delete_key_by_did(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path(key_id): Path<String>,
        payload: Result<Json<StringHelper>, JsonRejection>,
    ) -> AppResult<Json<did::Model>> {
        let payload = extract_payload(payload)?;
        Ok(Json(
            core.delete_key(DidSearch::Did(payload.data().to_string()), key_id)
                .await?,
        ))
    }

    async fn default_key_by_did(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path(key_id): Path<String>,
        payload: Result<Json<StringHelper>, JsonRejection>,
    ) -> AppResult<Json<did::Model>> {
        let payload = extract_payload(payload)?;
        Ok(Json(
            core.set_default_key(DidSearch::Did(payload.data().to_string()), key_id)
                .await?,
        ))
    }

    async fn add_key_by_id(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path((id, key_id)): Path<(String, String)>,
    ) -> AppResult<Json<did::Model>> {
        Ok(Json(core.add_key(DidSearch::Id(id), key_id).await?))
    }

    async fn delete_key_by_id(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path((id, key_id)): Path<(String, String)>,
    ) -> AppResult<Json<did::Model>> {
        Ok(Json(core.delete_key(DidSearch::Id(id), key_id).await?))
    }

    async fn default_key_by_id(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path((id, key_id)): Path<(String, String)>,
    ) -> AppResult<Json<did::Model>> {
        Ok(Json(core.set_default_key(DidSearch::Id(id), key_id).await?))
    }

    async fn get_by_did(
        State(core): State<Arc<dyn DidManagerModule>>,
        payload: Result<Json<StringHelper>, JsonRejection>,
    ) -> AppResult<Json<did::Model>> {
        let payload = extract_payload(payload)?;
        Ok(Json(
            core.get_by_id(DidSearch::Did(payload.data().to_string()))
                .await?,
        ))
    }

    async fn get_by_id(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path(id): Path<String>,
    ) -> AppResult<Json<did::Model>> {
        Ok(Json(core.get_by_id(DidSearch::Id(id)).await?))
    }

    async fn delete_by_did(
        State(core): State<Arc<dyn DidManagerModule>>,
        payload: Result<Json<StringHelper>, JsonRejection>,
    ) -> AppResult<StatusCode> {
        let payload = extract_payload(payload)?;
        core.delete(DidSearch::Did(payload.data().to_string()))
            .await?;
        Ok(StatusCode::OK)
    }

    async fn delete_by_id(
        State(core): State<Arc<dyn DidManagerModule>>,
        Path(id): Path<String>,
    ) -> AppResult<StatusCode> {
        core.delete(DidSearch::Id(id)).await?;
        Ok(StatusCode::OK)
    }
}
