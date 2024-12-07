use loco_rs::schema::table_auto_tz;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto_tz(Configs::Table)
                    .col(pk_auto(Configs::Id))
                    .col(uuid_uniq(Configs::Pid))
                    .col(integer(Configs::UserId))
                    .col(integer(Configs::MachineId))
                    .col(json(Configs::Data))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-configs-user_ids")
                            .from(Configs::Table, Configs::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-configs-machine_ids")
                            .from(Configs::Table, Configs::MachineId)
                            .to(Machines::Table, Machines::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Configs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Configs {
    Table,
    Id,
    Pid,
    UserId,
    MachineId,
    Data,
    
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
#[derive(DeriveIden)]
enum Machines {
    Table,
    Id,
}
