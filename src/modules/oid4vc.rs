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

use ymir::types::verification::{PresentationRequest, Requested};
use crate::services::{HasOid4Vci, HasOid4Vp, HasRepo};
use async_trait::async_trait;
use tracing::{info, warn};
use chrono::Utc;
use ymir::data::entities::wallet::vc;
use ymir::errors::{BadFormat, Errors, Outcome};
use ymir::types::dids::Did;
use ymir::types::issuance::VcBody;
use ymir::types::issuance::VcCredOffer;
use ymir::capabilities::{VcStatusCheck, Verifier};
use ymir::types::jwt::Jwt;
use ymir::types::vcs::VcPayload;
use ymir::types::vcs::VPDef;
use ymir::types::vcs::{VcFormat, VcType};
use ymir::types::verification::{CredentialQuery, DcqlQuery};
use ymir::types::wallet::{
    HeldCredential, KeyRef, PresentationChoice, PresentationPreview,
};
use std::collections::HashMap;

#[async_trait]
pub trait Oid4VcModule: HasRepo + HasOid4Vp + HasOid4Vci + Send + Sync + 'static {
    // ===== OID4VCI ===============================================================================

    async fn process_oid4vci(&self, uri: &str) -> Outcome<()> {
        let offer = self.oid4vci().resolve_offer(uri).await?;
        self.vci_accept_offer(offer).await
    }

    async fn vci_resolve_offer(&self, uri: &str) -> Outcome<VcCredOffer> {
        self.oid4vci().resolve_offer(uri).await
    }

    async fn vci_accept_offer(&self, offer: VcCredOffer) -> Outcome<()> {
        let plans = self.oid4vci().accept_offer(&offer).await?;
        for plan in plans {
            let doc = match &plan.vc_body {
                VcBody::Jwt(jwt) => {
                    let claims = Jwt::<VcPayload>::parse(jwt)?;
                    claims.unverified_claims().vc_doc().clone()
                }
                VcBody::JsonLd(value) => serde_json::from_value(value.clone())?,
            };

            let vc_format = match &plan.vc_body {
                VcBody::Jwt(_) => VcFormat::JwtVcJson,
                VcBody::JsonLd(_) => VcFormat::LdpVc,
            };

            let model = vc::Model {
                id: doc.id().to_string(),
                vc_body: plan.vc_body,
                vc_type: doc
                    .specialized_type()
                    .unwrap_or(VcType::Other("Unknown".to_string())),
                vc_format,
                holder_did: doc.holder_did().unwrap_or("Unknown").to_string(),
                issuer_did: doc.issuer.id().to_string(),
                parsed_document: serde_json::to_value(&doc)?,
                valid_until: doc.valid_until,
                added_on: Utc::now(),
            };

            self.repo().vcs().create(model).await?;
        }
        Ok(())
    }

    // ===== OID4VP ================================================================================
    async fn process_oid4vp(&self, uri: &str) -> Outcome<Option<String>> {
        self.vp_present(uri, None).await
    }

    async fn vp_resolve_request(&self, uri: &str) -> Outcome<PresentationRequest> {
        self.oid4vp(uri)?.resolve_request(uri).await
    }

    async fn vp_refuse(&self, uri: &str) -> Outcome<()> {
        let oid4vp = self.oid4vp(uri)?;
        let request = oid4vp.resolve_request(uri).await?;
        oid4vp.refuse(&request, "access_denied", "The holder declined to present").await
    }

    /// What the wallet holds for the DCQL request at `uri`, and what it presents by default.
    async fn vp_preview(&self, uri: &str) -> Outcome<PresentationPreview> {
        let request = self.oid4vp(uri)?.resolve_request(uri).await?;
        let Requested::Dcql(query) = &request.requested else {
            return Err(Errors::not_impl("Only an OpenID4VP 1.0 request has a preview", None));
        };
        let available = self.vp_available(query).await?;
        let (default, missing) = match default_choice(query, &available) {
            Ok(selected) => (Some(choice_of(&selected)), None),
            Err(e) => (None, Some(e.reason().to_string())),
        };
        Ok(PresentationPreview {
            available: available
                .iter()
                .map(|(id, answers)| (id.clone(), answers.usable.iter().map(held).collect()))
                .collect(),
            default,
            missing,
        })
    }

    /// Presents to the verifier of `uri`, with the OpenID4VP version that handles it, the
    /// credentials of `choice` or, without it, the wallet's own. When the wallet cannot (no
    /// credential matches, a choice that does not answer the request, no holder DID or key), it
    /// tells the verifier so (`refuse`) before failing, so the verifier does not wait for a
    /// presentation.
    async fn vp_present(&self, uri: &str, choice: Option<&PresentationChoice>) -> Outcome<Option<String>> {
        let oid4vp = self.oid4vp(uri)?;
        let request = oid4vp.resolve_request(uri).await?;
        let selected = match choice {
            Some(choice) => self.vp_chosen(&request.requested, choice).await,
            None => self.vp_select(&request.requested).await,
        };
        match selected {
            Ok((selected_vcs, did, key_ref)) => {
                oid4vp.present(&request, &selected_vcs, &did, &key_ref).await
            }
            Err(e) => {
                if let Err(refusal) = oid4vp.refuse(&request, "access_denied", e.reason()).await {
                    warn!("Could not tell the verifier this wallet will not present: {refusal}");
                }
                Err(e)
            }
        }
    }

    /// The holder's `choice` for `requested`, if it answers it, with the holder DID and its
    /// default key.
    async fn vp_chosen(
        &self,
        requested: &Requested,
        choice: &PresentationChoice,
    ) -> Outcome<(Vec<(String, Vec<vc::Model>)>, Did, KeyRef)> {
        let Requested::Dcql(query) = requested else {
            return Err(Errors::not_impl("Only an OpenID4VP 1.0 request can be chosen for", None));
        };
        let selected = chosen(query, &self.vp_available(query).await?, choice)?;
        let (did, key_ref) = self.vp_holder(&selected).await?;
        Ok((selected, did, key_ref))
    }

    /// The credentials for each requested item (by id), with the holder DID and its default key.
    async fn vp_select(
        &self,
        requested: &Requested,
    ) -> Outcome<(Vec<(String, Vec<vc::Model>)>, Did, KeyRef)> {
        let selected = match requested {
            Requested::Definition(vpd) => self.vp_select_definition(vpd).await?,
            Requested::Dcql(query) => self.vp_select_dcql(query).await?,
        };
        let (did, key_ref) = self.vp_holder(&selected).await?;
        Ok((selected, did, key_ref))
    }

    /// The wallet's own answer: for each credential query, the newest usable credential (all with
    /// `multiple`); with `credential_sets` (§6.4.2), the queries of the first option of each set it
    /// meets, and a required set with none fails.
    async fn vp_select_dcql(&self, query: &DcqlQuery) -> Outcome<Vec<(String, Vec<vc::Model>)>> {
        default_choice(query, &self.vp_available(query).await?)
    }

    /// For each credential query, the stored credentials that answer it (same format, all the
    /// types of one of its `type_values` sets, not expired, nor revoked or suspended), newest first.
    async fn vp_available(&self, query: &DcqlQuery) -> Outcome<HashMap<String, Answers>> {
        let mut stored = self.repo().vcs().get_all(None, None).await?;
        // TODO: with a label in the request, present the LegalPerson it lists, not the newest (plan row 24).
        stored.sort_by(|a, b| b.added_on.cmp(&a.added_on));
        let now = Utc::now();
        let mut available = HashMap::new();
        for credential in &query.credentials {
            let matching: Vec<&vc::Model> = stored
                .iter()
                .filter(|vc| answers(credential, vc) && vc.valid_until.map_or(true, |exp| now < exp))
                .collect();
            let mut usable = Vec::new();
            for vc in &matching {
                if is_withdrawn(vc).await {
                    info!("Skipping vc {}: its issuer revoked or suspended it", vc.id);
                    continue;
                }
                usable.push((*vc).clone());
            }
            available.insert(credential.id.clone(), Answers { usable, matched: matching.len() });
        }
        Ok(available)
    }

    /// For each input descriptor, the newest stored credential that matches it and is not
    /// expired, revoked or suspended.
    async fn vp_select_definition(&self, vpd: &VPDef) -> Outcome<Vec<(String, Vec<vc::Model>)>> {
        let now = Utc::now();
        let mut selected = Vec::new();
        for desc in &vpd.input_descriptors {
            let mut matching: Vec<vc::Model> = self
                .repo()
                .vcs()
                .filter_by_desc(desc)
                .await?
                .into_iter()
                .filter(|vc| vc.valid_until.map_or(true, |exp| now < exp))
                .collect();
            matching.sort_by(|a, b| b.added_on.cmp(&a.added_on));
            let what = format!("input descriptor '{}'", desc.id);
            let chosen = usable(matching, false, &what).await?;
            selected.push((desc.id.clone(), chosen));
        }
        Ok(selected)
    }

    /// The first of our DIDs among the selected credentials' subjects (numbers and labels have
    /// their own `#cs` one), else the default DID; with its default key.
    async fn vp_holder(&self, selected: &[(String, Vec<vc::Model>)]) -> Outcome<(Did, KeyRef)> {
        let mut subjects: Vec<String> = Vec::new();
        for (_, vcs) in selected {
            subjects.extend(vcs.iter().map(|vc| vc.holder_did.clone()));
        }
        if subjects.is_empty() {
            return Err(Errors::missing_resource("vc", "The request asks for no credential", None));
        }
        for subject in &subjects {
            let Ok(did) = Did::parse(subject) else { continue };
            if let Ok(did_model) = self.repo().dids().get_by_did(did.id()).await {
                return Ok((did, did_model.default_key));
            }
        }
        let did_model = self.repo().dids().get_default().await?;
        Ok((Did::parse(&did_model.did)?, did_model.default_key))
    }
}

/// The usable credentials that answer a credential query, and how many matched before the
/// revoked or suspended ones were left out.
pub struct Answers {
    usable: Vec<vc::Model>,
    matched: usize,
}

/// The wallet's own answer to `credential`: its newest usable credential, or all with `multiple`.
fn pick(credential: &CredentialQuery, available: &HashMap<String, Answers>) -> Outcome<Vec<vc::Model>> {
    let answers = available.get(&credential.id);
    let usable = answers.map(|a| a.usable.as_slice()).unwrap_or_default();
    if usable.is_empty() {
        let what = format!("credential query '{}'", credential.id);
        let reason = match answers.map_or(0, |a| a.matched) {
            0 => format!("Missing vc that matches {what}"),
            _ => format!("Every vc that matches {what} is revoked or suspended"),
        };
        return Err(Errors::missing_resource("vc", reason, None));
    }
    let take = if credential.multiple { usable.len() } else { 1 };
    Ok(usable[..take].to_vec())
}

/// The wallet's own answer to `query` (see `vp_select_dcql`).
fn default_choice(
    query: &DcqlQuery,
    available: &HashMap<String, Answers>,
) -> Outcome<Vec<(String, Vec<vc::Model>)>> {
    let Some(sets) = &query.credential_sets else {
        return query
            .credentials
            .iter()
            .map(|credential| Ok((credential.id.clone(), pick(credential, available)?)))
            .collect();
    };

    let mut selected: Vec<(String, Vec<vc::Model>)> = Vec::new();
    for set in sets {
        let mut reasons = Vec::new();
        let mut met = None;
        for option in &set.options {
            match answer_option(query, option, available) {
                Ok(answers) => {
                    met = Some(answers);
                    break;
                }
                Err(e) => reasons.push(e.reason().to_string()),
            }
        }
        match met {
            Some(answers) => {
                for (id, vcs) in answers {
                    if !selected.iter().any(|(chosen, _)| *chosen == id) {
                        selected.push((id, vcs));
                    }
                }
            }
            None if set.required => {
                return Err(Errors::missing_resource(
                    "vc",
                    format!("No option of a required credential set is met: {}", reasons.join("; ")),
                    None,
                ));
            }
            None => {}
        }
    }
    Ok(selected)
}

/// The wallet's own answer to every credential query of one `credential_sets` option.
fn answer_option(
    query: &DcqlQuery,
    option: &[String],
    available: &HashMap<String, Answers>,
) -> Outcome<Vec<(String, Vec<vc::Model>)>> {
    option
        .iter()
        .map(|id| {
            let credential = query.query(id).ok_or_else(|| {
                Errors::format(
                    BadFormat::Received,
                    format!("credential_sets names an unknown credential query '{id}'"),
                    None,
                )
            })?;
            Ok((id.clone(), pick(credential, available)?))
        })
        .collect()
}

/// The holder's `choice`, if it answers `query`: each credential usable for its query, one per
/// query unless `multiple`, and every query answered or, with `credential_sets`, an option of
/// each required set.
fn chosen(
    query: &DcqlQuery,
    available: &HashMap<String, Answers>,
    choice: &PresentationChoice,
) -> Outcome<Vec<(String, Vec<vc::Model>)>> {
    let refused = |reason: String| Errors::format(BadFormat::Received, reason, None);
    let mut selected = Vec::new();
    for (id, vc_ids) in choice {
        let credential = query
            .query(id)
            .ok_or_else(|| refused(format!("The request has no credential query '{id}'")))?;
        if vc_ids.is_empty() || (!credential.multiple && vc_ids.len() > 1) {
            return Err(refused(format!("Credential query '{id}' takes one credential")));
        }
        let usable = available.get(id).map(|a| a.usable.as_slice()).unwrap_or_default();
        let vcs = vc_ids
            .iter()
            .map(|vc_id| {
                usable.iter().find(|vc| vc.id == *vc_id).cloned().ok_or_else(|| {
                    refused(format!("vc {vc_id} does not answer credential query '{id}'"))
                })
            })
            .collect::<Outcome<Vec<_>>>()?;
        selected.push((id.clone(), vcs));
    }

    let answered = |id: &String| choice.contains_key(id);
    match &query.credential_sets {
        None => {
            if let Some(left) = query.credentials.iter().find(|c| !answered(&c.id)) {
                return Err(refused(format!("The choice leaves credential query '{}' unanswered", left.id)));
            }
        }
        Some(sets) => {
            if sets.iter().any(|set| set.required && !set.options.iter().any(|o| o.iter().all(answered))) {
                return Err(refused("The choice meets no option of a required credential set".to_string()));
            }
        }
    }
    Ok(selected)
}

/// The credential ids of an answer, by credential query.
fn choice_of(selected: &[(String, Vec<vc::Model>)]) -> PresentationChoice {
    selected
        .iter()
        .map(|(id, vcs)| (id.clone(), vcs.iter().map(|vc| vc.id.clone()).collect()))
        .collect()
}

fn held(vc: &vc::Model) -> HeldCredential {
    HeldCredential {
        id: vc.id.clone(),
        types: document_types(vc),
    }
}

/// The credentials of `matching`, in order, that the issuer has not revoked or suspended: the
/// first, or all of them with `multiple`.
async fn usable(matching: Vec<vc::Model>, multiple: bool, what: &str) -> Outcome<Vec<vc::Model>> {
    let found = matching.len();
    let mut chosen = Vec::new();
    for vc in matching {
        if is_withdrawn(&vc).await {
            info!("Skipping vc {}: its issuer revoked or suspended it", vc.id);
            continue;
        }
        chosen.push(vc);
        if !multiple {
            break;
        }
    }
    if chosen.is_empty() {
        let reason = match found {
            0 => format!("Missing vc that matches {what}"),
            _ => format!("Every vc that matches {what} is revoked or suspended"),
        };
        return Err(Errors::missing_resource("vc", reason, None));
    }
    Ok(chosen)
}

/// Whether the issuer has revoked or suspended `vc`. One whose status cannot be told counts as
/// usable: the verifier has the last word.
async fn is_withdrawn(vc: &vc::Model) -> bool {
    let VcBody::Jwt(token) = &vc.vc_body else {
        return false;
    };
    let Ok(jwt) = Jwt::<VcPayload>::parse(token) else {
        return false;
    };
    match Verifier::verify_enveloped(jwt, None).await {
        Ok(verified) => VcStatusCheck::is_withdrawn(&verified).await,
        Err(_) => false,
    }
}

/// Whether a stored credential answers a credential query: its format and, from the `type` of
/// its document, one of the query's type sets (DCQL `meta.type_values`).
fn answers(credential: &CredentialQuery, vc: &vc::Model) -> bool {
    vc.vc_format == credential.format && credential.matches_types(&document_types(vc))
}

/// The `type` of a stored credential's document.
fn document_types(vc: &vc::Model) -> Vec<String> {
    match vc.parsed_document.get("type") {
        Some(serde_json::Value::Array(types)) => types
            .iter()
            .filter_map(|t| t.as_str().map(str::to_string))
            .collect(),
        Some(serde_json::Value::String(t)) => vec![t.clone()],
        _ => Vec::new(),
    }
}
