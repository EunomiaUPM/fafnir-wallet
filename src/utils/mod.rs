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
use reqwest::Response;
use serde::de::DeserializeOwned;
use url::Url;
use ymir::errors::{Errors, Outcome, PetitionFailure};
use ymir::utils::ResponseExt;

/// Swaps the URL's host when it is exactly a loopback (in Docker) or `host.docker.internal`
/// (outside), so names like `provider.localhost` stay as they are.
pub fn normalize_host(host: impl Into<String>, is_docker: bool) -> String {
    let host = host.into();
    let Some(current) = Url::parse(&host)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
    else {
        return host;
    };
    let swapped = match (is_docker, current.as_str()) {
        (true, "localhost" | "127.0.0.1") => "host.docker.internal",
        (false, "host.docker.internal") => "localhost",
        _ => return host,
    };
    // Only the host changes; the rest of the URL is kept as written (no trailing `/` added).
    host.replacen(&format!("://{current}"), &format!("://{swapped}"), 1)
}

pub async fn parse_or_fail<T: DeserializeOwned>(
    res: Response,
    url: &str,
    method: &str,
) -> Outcome<T> {
    if res.status().is_success() {
        res.parse_json().await
    } else {
        Err(Errors::petition(
            url,
            method,
            Some(res.status()),
            PetitionFailure::HttpStatus(res.status()),
            "unexpected http status",
            None,
        ))
    }
}
