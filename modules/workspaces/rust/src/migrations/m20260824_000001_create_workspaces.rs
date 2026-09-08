use module_host::migration::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Workspaces::Table)
                    .col(
                        ColumnDef::new(Workspaces::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Workspaces::CanonicalRoot)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Workspaces::DisplayName).string().not_null())
                    .col(
                        ColumnDef::new(Workspaces::LastOpenedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Workspaces::CreatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Workspaces::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
    Id,
    CanonicalRoot,
    DisplayName,
    LastOpenedAt,
    CreatedAt,
}
