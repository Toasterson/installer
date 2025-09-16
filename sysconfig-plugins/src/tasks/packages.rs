//! Package management (publishers and images) placeholder.

use serde_json::Value;

#[derive(Default)]
pub struct Packages;

impl crate::TaskHandler for Packages {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        Ok(String::new())
    }
}
