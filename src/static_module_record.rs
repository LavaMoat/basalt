use std::collections::HashMap;

use serde::{Serialize, Deserialize};

pub type LiveExport = (String, bool);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticModuleRecord {
    pub export_alls: Vec<String>,
    pub imports: HashMap<String, Vec<String>>,
    pub live_export_map: HashMap<String, LiveExport>,
    pub fixed_export_map: HashMap<String, Vec<String>>,
}
