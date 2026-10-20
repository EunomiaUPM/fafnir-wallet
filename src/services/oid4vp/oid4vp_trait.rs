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
use ymir::data::entities::wallet::vc;
use ymir::errors::Outcome;
use ymir::types::dids::Did;
use ymir::types::wallet::KeyRef;

use ymir::types::verification::PresentationRequest;

/// One OpenID4VP version on the wallet's side; the wallet picks the one that [`handles`] a URI.
///
/// [`handles`]: Oid4vpTrait::handles
#[async_trait]
pub trait Oid4vpTrait: Send + Sync + 'static {
    /// Whether `uri` is a request of this version.
    fn handles(&self, uri: &str) -> bool;

    /// Reads the request of an `openid4vp://` URI.
    async fn resolve_request(&self, uri: &str) -> Outcome<PresentationRequest>;

    /// Presents the credentials chosen for each requested item (input descriptor or credential
    /// query, by id), signed with `holder_key`. Returns where the verifier sends the user next.
    async fn present(
        &self,
        request: &PresentationRequest,
        selected_vcs: &[(String, Vec<vc::Model>)],
        holder_did: &Did,
        holder_key: &KeyRef,
    ) -> Outcome<Option<String>>;

    /// Tells the verifier this wallet will not present (`error`, e.g. `access_denied`), so it
    /// closes the request instead of waiting.
    async fn refuse(
        &self,
        request: &PresentationRequest,
        error: &str,
        description: &str,
    ) -> Outcome<()>;
}
