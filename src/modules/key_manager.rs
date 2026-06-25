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
use async_trait::async_trait;
use chrono::Utc;
use ymir::data::entities::wallet::key;
use ymir::errors::Outcome;
use ymir::services::HasVault;
use ymir::services::vault::VaultTrait;
use ymir::types::secrets::PemHelper;

#[async_trait]
pub trait KeyManagerModule: HasVault + HasRepo + Send + Sync + 'static {
    async fn get_all(&self) -> Outcome<Vec<key::Model>> {
        self.repo().keys().get_all(None, None).await
    }

    async fn get_by_id(&self, id: String) -> Outcome<key::Model> {
        self.repo().keys().get_by_id(&id).await
    }

    async fn create(&self, plan: key::Plan) -> Outcome<key::Model> {
        let key = PemHelper::priv_from_pem(&plan.pem)?;
        self.vault().write(None, &plan.id, &key).await?;

        let key_model = key::Model {
            id: plan.id,
            r#default: plan.r#default,
            alias: plan.alias,
            kty: key.kty().clone(),
            crv: key.crv().cloned(),
            created_at: Utc::now(),
        };

        self.repo().keys().create(key_model).await
    }

    async fn delete(&self, id: String) -> Outcome<()> {
        self.repo().keys().delete(&id).await
    }
}
