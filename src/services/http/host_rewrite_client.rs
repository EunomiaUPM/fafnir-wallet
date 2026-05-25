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

use async_trait::async_trait;
use axum::http::HeaderMap;
use reqwest::{Response, Url};
use tracing::debug;
use ymir::errors::Outcome;
use ymir::services::client::ClientTrait;
use ymir::types::http::Body;

/// Wrapper sobre `ClientTrait` que reescribe `127.0.0.1` y `localhost`
/// → `host.docker.internal` en cada URL antes de delegar al cliente
/// real. Útil cuando la wallet corre dentro de un contenedor y los
/// servicios (issuer/verifier) emiten URIs apuntando a host local.
///
/// Si `enabled = false` actúa como passthrough transparente.
pub struct HostRewriteClient {
    inner: Arc<dyn ClientTrait>,
    enabled: bool,
}

impl HostRewriteClient {
    pub fn new(inner: Arc<dyn ClientTrait>, enabled: bool) -> Self {
        Self { inner, enabled }
    }

    fn rewrite(&self, url: &str) -> String {
        if !self.enabled {
            return url.to_string();
        }
        // Solo tocamos host, no las apariciones en queries (p.ej. dentro
        // de un response_uri url-encoded). Para eso parseamos la URL.
        let new_url = match Url::parse(url) {
            Ok(mut u) => {
                let host_changed = match u.host_str() {
                    Some("127.0.0.1") | Some("localhost") => {
                        u.set_host(Some("host.docker.internal")).ok();
                        true
                    }
                    _ => false,
                };
                if host_changed {
                    u.to_string()
                } else {
                    url.to_string()
                }
            }
            Err(_) => url.to_string(),
        };
        if new_url != url {
            debug!("HostRewriteClient: {} → {}", url, new_url);
        }
        new_url
    }
}

#[async_trait]
impl ClientTrait for HostRewriteClient {
    async fn get(&self, url: &str, headers: Option<HeaderMap>) -> Outcome<Response> {
        self.inner.get(&self.rewrite(url), headers).await
    }

    async fn post(&self, url: &str, headers: Option<HeaderMap>, body: Body) -> Outcome<Response> {
        self.inner.post(&self.rewrite(url), headers, body).await
    }

    async fn put(&self, url: &str, headers: Option<HeaderMap>, body: Body) -> Outcome<Response> {
        self.inner.put(&self.rewrite(url), headers, body).await
    }

    async fn delete(&self, url: &str, headers: Option<HeaderMap>, body: Body) -> Outcome<Response> {
        self.inner.delete(&self.rewrite(url), headers, body).await
    }
}
