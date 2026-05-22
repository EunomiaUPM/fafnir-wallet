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

use sea_orm::DatabaseConnection;
use sea_orm_migration::{MigrationTrait, MigratorTrait};
use ymir::errors::{Errors, Outcome};

use crate::data::Migrator;

pub struct AuthorityMigration;

impl MigratorTrait for AuthorityMigration {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut migrations: Vec<Box<dyn MigrationTrait>> = vec![];
        let mut authority = Migrator::migrations();

        migrations.append(&mut authority);
        migrations
    }
}

impl AuthorityMigration {
    pub async fn run(db_connection: &DatabaseConnection) -> Outcome<()> {
        Self::refresh(db_connection)
            .await
            .map_err(|e| Errors::db("Error migrating data", Some(Box::new(e))))
    }
}
