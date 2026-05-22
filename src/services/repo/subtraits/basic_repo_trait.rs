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

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use ymir::errors::{BadFormat, Errors, Outcome};
use ymir::utils::{HasId, read_json, write_json};

#[async_trait]
pub trait BasicRepoTrait<T, Req>: Send + Sync + 'static
where
    T: Serialize + DeserializeOwned + HasId + Send + Sync + 'static,
    Req: Send + Sync + 'static,
{
    fn dir(&self) -> &Path;

    async fn build_entry(&self, req: Req) -> Outcome<T>;

    async fn get_all(&self) -> Outcome<Vec<T>> {
        let dir = self.dir();
        let read_dir = std::fs::read_dir(dir).map_err(|e| {
            Errors::read(
                dir.display().to_string(),
                "read_dir failed",
                Some(Box::new(e)),
            )
        })?;
        let mut items = Vec::new();
        for entry in read_dir {
            let entry = entry.map_err(|e| {
                Errors::read(
                    dir.display().to_string(),
                    "read_dir entry failed",
                    Some(Box::new(e)),
                )
            })?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            items.push(read_json(&path)?);
        }
        Ok(items)
    }

    async fn get_by_id(&self, id: &str) -> Outcome<T> {
        let path = self.path_for(id);
        if !path.exists() {
            return Err(Errors::missing_resource(
                id,
                format!("no entry with id {id}"),
                None,
            ));
        }
        read_json(&path)
    }

    async fn create(&self, req: Req) -> Outcome<T> {
        let entry = self.build_entry(req).await?;
        let path = self.path_for(entry.id());
        if path.exists() {
            return Err(Errors::format(
                BadFormat::Sent,
                format!("entry with id {} already exists", entry.id()),
                None,
            ));
        }
        write_json(&path, &entry)?;
        Ok(entry)
    }

    async fn update(&self, item: T) -> Outcome<T> {
        let path = self.path_for(item.id());
        if !path.exists() {
            return Err(Errors::missing_resource(
                item.id(),
                format!("cannot update missing entry {}", item.id()),
                None,
            ));
        }
        write_json(&path, &item)?;
        Ok(item)
    }

    async fn save(&self, item: T) -> Outcome<T> {
        let path = self.path_for(item.id());
        write_json(&path, &item)?;
        Ok(item)
    }

    async fn delete(&self, id: &str) -> Outcome<()> {
        let path = self.path_for(id);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Err(Errors::missing_resource(
                id,
                format!("cannot delete missing entry {id}"),
                None,
            )),
            Err(e) => Err(Errors::write(
                path.display().to_string(),
                "remove_file failed",
                Some(Box::new(e)),
            )),
        }
    }

    async fn exists(&self, id: &str) -> bool {
        self.path_for(id).exists()
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir().join(format!("{id}.json"))
    }
}
