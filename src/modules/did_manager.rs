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

use crate::services::HasRepo;
use crate::types::DidSearch;
use async_trait::async_trait;
use ymir::data::entities::wallet::{did, key};
use ymir::errors::{BadFormat, Errors, Outcome};
use ymir::services::vault::VaultTrait;
use ymir::services::{HasIdentity, HasVault};
use ymir::types::dids::{DidDocument, DidType};
use ymir::types::keys::PrivateKey;
use ymir::types::secrets::PemHelper;
use ymir::types::wallet::KeyRef;

#[async_trait]
pub trait DidManagerModule: HasIdentity + HasVault + HasRepo + Send + Sync + 'static {
    async fn get_all(&self) -> Outcome<Vec<did::Model>> {
        self.repo().dids().get_all(None, None).await
    }

    async fn create(&self, entry: did::Plan) -> Outcome<did::Model> {
        if entry.keys.is_empty() {
            return Err(Errors::format(
                BadFormat::Received,
                "Missing keys to associate did",
                None,
            ));
        }

        let mut key_models = Vec::with_capacity(entry.keys.len());

        for id in &entry.keys {
            let model = self.repo().keys().get_by_id(id).await?;
            key_models.push(model)
        }

        let did = entry.builder.build()?;

        let matched = match_keys_to_fragments(did.r#type(), key_models);

        let mut keys_ref = Vec::with_capacity(matched.len());
        let mut mapa = Vec::with_capacity(matched.len());

        for (key_model, frag) in matched {
            keys_ref.push(KeyRef::new(key_model.id.clone(), frag.clone()));
            let helper: PemHelper = self.vault().read(None, &key_model.id).await?;
            let priv_key = PrivateKey::from_safe_pem(helper.pem(), helper.kty(), helper.crv())?;
            mapa.push((priv_key, frag));
        }

        let mut did_document = DidDocument::base(&did, mapa);
        if let Some(services) = &entry.service {
            did_document.add_services(services.clone());
        };

        let is_first = self.repo().dids().get_all(None, None).await?.is_empty();

        let model = did::Model {
            id: uuid::Uuid::new_v4().to_string(),
            did: did.id().to_string(),
            alias: entry.alias,
            r#default: is_first,
            r#type: did.r#type(),
            default_key: keys_ref.first().unwrap().clone(),
            keys: keys_ref,
            did_document,
            service: entry.service,
        };

        let model = self.repo().dids().create(model).await?;

        if is_first {
            self.identity().save_identity(&model).await?;
        }
        Ok(model)
    }

    async fn get_default(&self) -> Outcome<did::Model> {
        self.repo().dids().get_default().await
    }

    async fn set_default(&self, id: DidSearch) -> Outcome<did::Model> {
        let model = match id {
            DidSearch::Id(id) => self.repo().dids().set_default_id(&id).await?,
            DidSearch::Did(did) => self.repo().dids().set_default_by_did(&did).await?,
        };

        {
            self.identity().save_identity(&model).await?;
        }
        Ok(model)
    }

    async fn add_key(&self, id: DidSearch, key_id: String) -> Outcome<did::Model> {
        let mut did_model = self.get_by_id(id).await?;

        match &did_model.r#type {
            DidType::Jwk => {
                return Err(Errors::format(
                    BadFormat::Received,
                    "Unable to add a new key to a did:jwk, just create a new one",
                    None,
                ));
            }
            DidType::Web => {}
        }
        let frag = did_model.keys.len().to_string();

        let helper: PemHelper = self.vault().read(None, &key_id).await?;
        let pkey = PrivateKey::from_safe_pem(helper.pem(), helper.kty(), helper.crv())?;
        did_model.did_document.add_key(&pkey, Some(&frag));

        let key_ref = KeyRef::new(&key_id, frag);
        did_model.keys.push(key_ref);

        self.repo().dids().update(did_model).await
    }

    async fn delete_key(&self, id: DidSearch, key_id: String) -> Outcome<did::Model> {
        let mut did_model = self.get_by_id(id).await?;

        match &did_model.r#type {
            DidType::Jwk => {
                return Err(Errors::format(
                    BadFormat::Received,
                    "Unable to delete a key to a did:jwk, just delete it completely",
                    None,
                ));
            }
            DidType::Web => {}
        }

        let size = did_model.keys.len();

        if size == 1 {
            return Err(Errors::format(
                BadFormat::Received,
                "Cannot delete a key from a did that only has 1",
                None,
            ));
        };

        let mut removed_frags = Vec::new();

        did_model.keys.retain(|key_ref| {
            if key_ref.internal() == key_id {
                removed_frags.push(key_ref.fragment().to_string());
                false
            } else {
                true
            }
        });

        let vm_frag = removed_frags.first().ok_or_else(|| {
            Errors::format(
                BadFormat::Received,
                "Did does not contain the key to delete",
                None,
            )
        })?;

        did_model.did_document.delete_key(vm_frag);

        if key_id == did_model.default_key.internal() {
            did_model.default_key = did_model.keys.first().unwrap().clone();
        }

        let did_model = self.repo().dids().update(did_model).await?;

        if did_model.r#default {
            self.identity().save_identity(&did_model).await?;
        }

        Ok(did_model)
    }

    async fn set_default_key(&self, id: DidSearch, key: String) -> Outcome<did::Model> {
        let mut did_model = self.get_by_id(id).await?;

        let selected_key = did_model
            .keys
            .iter()
            .find(|key_ref| key_ref.internal() == key)
            .cloned()
            .ok_or_else(|| {
                Errors::format(
                    BadFormat::Received,
                    "Missing key inside did, associate it first to the did",
                    None,
                )
            })?;

        did_model.default_key = selected_key;

        let did_model = self.repo().dids().update(did_model).await?;

        if did_model.r#default {
            self.identity().save_identity(&did_model).await?;
        }

        Ok(did_model)
    }

    async fn get_by_id(&self, id: DidSearch) -> Outcome<did::Model> {
        match id {
            DidSearch::Id(id) => self.repo().dids().get_by_id(&id).await,
            DidSearch::Did(did) => self.repo().dids().get_by_did(&did).await,
        }
    }

    async fn delete(&self, id: DidSearch) -> Outcome<()> {
        let target = match &id {
            DidSearch::Id(id) => self.repo().dids().get_by_id(id).await?,
            DidSearch::Did(did) => self.repo().dids().get_by_did(did).await?,
        };

        match id {
            DidSearch::Id(id) => self.repo().dids().delete(&id).await?,
            DidSearch::Did(did) => self.repo().dids().delete_by_did(&did).await?,
        }

        if target.default {
            let models = self.repo().dids().get_all(None, None).await?;

            if models.is_empty() {
                self.identity().clear_identity().await?;
            } else {
                let first_model = models.first().unwrap();
                let promoted_model = self.repo().dids().set_default_id(&first_model.id).await?;
                self.identity().save_identity(&promoted_model).await?;
            }
        }
        Ok(())
    }
}

fn match_keys_to_fragments(did_type: DidType, keys: Vec<key::Model>) -> Vec<(key::Model, String)> {
    match did_type {
        DidType::Jwk => keys
            .into_iter()
            .take(1)
            .map(|k| (k, "0".to_string()))
            .collect(),
        DidType::Web => keys
            .into_iter()
            .enumerate()
            .map(|(i, k)| (k, format!("key-{i}")))
            .collect(),
    }
}
