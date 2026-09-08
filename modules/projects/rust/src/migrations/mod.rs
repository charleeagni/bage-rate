use module_host::migration::*;

mod m20260810_000001_create_projects;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260810_000001_create_projects::Migration)]
    }
}
