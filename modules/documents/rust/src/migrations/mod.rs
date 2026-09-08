use module_host::migration::*;

mod m20260824_000001_create_documents;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260824_000001_create_documents::Migration)]
    }
}
