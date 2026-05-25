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

use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::{Level, error, info};

use crate::core::Core;
use crate::core::traits::{CoreDidsTrait, CoreKeysTrait, CoreOid4vcTrait, CoreVcsTrait};
use crate::http::{DidRouter, KeyRouter, Oid4vcRouter, VcsRouter};

pub struct WalletRouter {
    core: Arc<Core>,
}

impl WalletRouter {
    pub fn new(core: Arc<Core>) -> Self {
        Self { core }
    }

    pub fn router(self) -> Router {
        let dids_core: Arc<dyn CoreDidsTrait> = self.core.clone();
        let keys_core: Arc<dyn CoreKeysTrait> = self.core.clone();
        let vcs_core: Arc<dyn CoreVcsTrait> = self.core.clone();
        let oid4vc_core: Arc<dyn CoreOid4vcTrait> = self.core.clone();

        let dids = DidRouter::new(dids_core).router();
        let keys = KeyRouter::new(keys_core).router();
        let vcs = VcsRouter::new(vcs_core).router();
        let oid4vc = Oid4vcRouter::new(oid4vc_core).router();

        Router::new()
            .nest("/dids", dids)
            .nest("/keys", keys)
            .nest("/vcs", vcs)
            .merge(oid4vc)
            .fallback(Self::fallback)
            .layer(
                TraceLayer::new_for_http()
                    .on_request(|req: &Request<_>, _span: &tracing::Span| {
                        info!("{} {}", req.method(), req.uri().path());
                    })
                    .on_response(DefaultOnResponse::new().level(Level::TRACE)),
            )
            .layer(CorsLayer::permissive())
    }

    async fn fallback() -> impl IntoResponse {
        error!("Wrong route");
        StatusCode::NOT_FOUND
    }
}
