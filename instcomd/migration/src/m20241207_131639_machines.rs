use loco_rs::schema::table_auto_tz;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto_tz(Machines::Table)
                    .col(pk_auto(Machines::Id))
                    .col(uuid_uniq(Machines::Pid))
                    .col(json_null(Machines::Disks))
                    .col(json(Machines::Interfaces))
                    .col(json_null(Machines::Devices))
                    .col(integer(Machines::UserId))
                    .col(date(Machines::StartDate))
                    .col(string(Machines::Name))
                    .col(uuid_null(Machines::UserData))
                    .col(uuid_null(Machines::MetaData))
                    .col(uuid_null(Machines::VendorData))
                    .col(uuid_null(Machines::NetworkConfig))
                    .col(uuid_null(Machines::Sysconfig))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-machines-user_ids")
                            .from(Machines::Table, Machines::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Machines::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Machines {
    Table,
    Id,
    Pid,
    Name,
    Disks,
    Interfaces,
    Devices,
    UserId,
    StartDate,
    UserData,
    MetaData,
    VendorData,
    NetworkConfig,
    Sysconfig,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
