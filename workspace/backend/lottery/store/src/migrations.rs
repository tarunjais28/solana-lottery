use diesel::pg::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use diesel::prelude::*;

use crate::DbConfig;

// diesel_migrations::embed_migrations!("./migrations");
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub fn run(db_config: &DbConfig) {
    let database_url = db_config.connection_string();
    let mut connection = PgConnection::establish(&database_url).expect("Connection failed");
    connection
        .run_pending_migrations(MIGRATIONS)
        .expect("Migrations failed");
}
