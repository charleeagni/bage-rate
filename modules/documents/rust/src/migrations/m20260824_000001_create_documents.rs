use module_host::migration::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Documents::Table)
                    // The identity is caller-supplied rather than
                    // auto-increment, so it is safe to expose through the
                    // create input.
                    .col(
                        ColumnDef::new(Documents::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Documents::TaskId).string().not_null())
                    .col(ColumnDef::new(Documents::Scope).string().not_null())
                    .col(ColumnDef::new(Documents::RootDir).string().not_null())
                    .col(ColumnDef::new(Documents::RelPath).string().not_null())
                    .col(ColumnDef::new(Documents::CreatedAt).string().not_null())
                    .col(ColumnDef::new(Documents::UpdatedAt).string().not_null())
                    // Absent until the first save, which is what makes the
                    // save operation a compare-and-swap rather than a write.
                    .col(ColumnDef::new(Documents::ContentDigest).string())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_documents_path")
                    .table(Documents::Table)
                    .col(Documents::RootDir)
                    .col(Documents::RelPath)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Documents::Table).to_owned())
            .await
    }
}

// The table name equals the Module name, which is what the prefix check
// wants: `design_documents` would normalise to `designdocuments` and fail
// generate.
#[derive(DeriveIden)]
enum Documents {
    Table,
    Id,
    TaskId,
    Scope,
    RootDir,
    RelPath,
    CreatedAt,
    UpdatedAt,
    ContentDigest,
}
