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

//! OpenID4VP on the wallet's side: one service per version, all behind [`Oid4vpTrait`].

pub mod oid4vp_1_0;
pub mod oid4vp_draft20;
mod oid4vp_trait;
mod presented;

pub use oid4vp_trait::Oid4vpTrait;
pub(crate) use presented::presented_vcs;
