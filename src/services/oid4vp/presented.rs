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

use ymir::data::entities::wallet::vc;
use ymir::errors::{BadFormat, Errors, Outcome};
use ymir::types::issuance::VcBody;
use ymir::types::vcs::W3cDataModelVersion;

/// The JWTs of `vcs` and the data model version they share, which their presentation follows.
pub(crate) fn presented_vcs(vcs: &[vc::Model]) -> Outcome<(Vec<String>, W3cDataModelVersion)> {
    let mut version = None;
    let mut jwts = Vec::with_capacity(vcs.len());
    for vc in vcs {
        let VcBody::Jwt(jwt) = &vc.vc_body else {
            return Err(Errors::not_impl(
                "Embedding a raw JSON VC in a JWT VP is not supported in the base impl",
                None,
            ));
        };
        let this = W3cDataModelVersion::from_document(&vc.parsed_document).ok_or_else(|| {
            Errors::format(
                BadFormat::Received,
                format!("VC '{}' has no known data model @context", vc.id),
                None,
            )
        })?;
        match version {
            Some(v) if v != this => {
                return Err(Errors::not_impl(
                    "Credentials of different data model versions cannot share a presentation",
                    None,
                ));
            }
            _ => version = Some(this),
        }
        jwts.push(jwt.clone());
    }
    Ok((jwts, version.unwrap_or_default()))
}
