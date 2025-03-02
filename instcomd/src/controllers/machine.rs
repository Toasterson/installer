#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::models::_entities::machines::{ActiveModel, Entity, Model};
use crate::models::_entities::{configs, machines};
use axum::debug_handler;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Params {
    pub disks: Option<serde_json::Value>,
    pub interfaces: serde_json::Value,
    pub devices: Option<serde_json::Value>,
    pub start_date: Date,
}

impl Params {
    fn update(&self, item: &mut ActiveModel) {
        item.disks = Set(self.disks.clone());
        item.interfaces = Set(self.interfaces.clone());
        item.devices = Set(self.devices.clone());
        item.start_date = Set(self.start_date.clone());
    }
}

async fn load_item(ctx: &AppContext, id: i32) -> Result<Model> {
    let item = Entity::find_by_id(id).one(&ctx.db).await?;
    item.ok_or_else(|| Error::NotFound)
}

async fn load_item_by_name(ctx: &AppContext, name: String) -> Result<Model> {
    let item = Entity::find()
        .filter(machines::Column::Name.eq(name))
        .one(&ctx.db)
        .await?;
    item.ok_or_else(|| Error::NotFound)
}

async fn load_config(ctx: &AppContext, pid: Option<Uuid>) -> Result<configs::Model> {
    let cfg_pid =
        pid.ok_or_else(|| Error::Message(String::from("no such config for machine exists")))?;
    let item = configs::Entity::find()
        .filter(configs::Column::Pid.eq(cfg_pid))
        .one(&ctx.db)
        .await?;
    item.ok_or_else(|| Error::NotFound)
}

#[debug_handler]
pub async fn add(State(ctx): State<AppContext>, Json(params): Json<Params>) -> Result<Response> {
    let mut item = ActiveModel {
        pid: ActiveValue::Set(Uuid::new_v4()),
        ..Default::default()
    };
    params.update(&mut item);
    let item = item.insert(&ctx.db).await?;
    format::json(item)
}

#[debug_handler]
pub async fn update(
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
    Json(params): Json<Params>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    let mut item = item.into_active_model();
    params.update(&mut item);
    let item = item.update(&ctx.db).await?;
    format::json(item)
}

#[debug_handler]
pub async fn remove(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {
    load_item(&ctx, id).await?.delete(&ctx.db).await?;
    format::empty()
}

#[debug_handler]
pub async fn get_one(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {
    format::json(load_item(&ctx, id).await?)
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigType {
    UserData,
    MetaData,
    VendorData,
    NetworkConfig,
    Sysconfig,
}

pub async fn get_config(
    Path((machine_name, config_type)): Path<(String, ConfigType)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let machine = load_item_by_name(&ctx, machine_name).await?;
    match config_type {
        ConfigType::UserData => {
            format::text(load_config(&ctx, machine.user_data).await?.data.as_str())
        }
        ConfigType::MetaData => {
            format::text(load_config(&ctx, machine.meta_data).await?.data.as_str())
        }
        ConfigType::VendorData => {
            format::text(load_config(&ctx, machine.vendor_data).await?.data.as_str())
        }
        ConfigType::NetworkConfig => format::text(
            load_config(&ctx, machine.network_config)
                .await?
                .data
                .as_str(),
        ),
        ConfigType::Sysconfig => {
            format::text(load_config(&ctx, machine.sysconfig).await?.data.as_str())
        }
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/machines/")
        .add("/", post(add))
        .add("{id}", get(get_one))
        .add("{id}", delete(remove))
        .add("{id}", put(update))
        .add("{id}", patch(update))
        .add("{machine_name}/configs/{config_type}", get(get_config))
}
