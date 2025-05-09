use instcomd::app::App;
use loco_rs::testing::prelude::*;
use loco_rs::{boot::run_task, task};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_can_seed_data() {
    let boot = boot_test::<App>().await.unwrap();

    assert!(run_task::<App>(
        &boot.app_context,
        Some(&"seed_data".to_string()),
        &task::Vars::default()
    )
    .await
    .is_ok());
}
