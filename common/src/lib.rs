use indexmap::{IndexMap, IndexSet};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum FromInspector {
    GetFeatures,
    #[serde(rename_all = "camelCase")]
    SetFeatures {
        selected_parameters: BTreeMap<String, BTreeSet<String>>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum ToInspector {
    #[serde(rename_all = "camelCase")]
    SetFeatures {
        selected_parameters: IndexMap<String, IndexMap<String, bool>>,
    },
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SerdeProfile {
    pub volume: Option<f32>,
    #[serde(default)]
    pub parameters: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SerdeProfiles {
    #[serde(default)]
    pub headphones: SerdeProfile,
    #[serde(default)]
    pub speakers: SerdeProfile,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SerdeCardSettings {
    #[serde(default)]
    pub selected_parameters: IndexMap<String, IndexSet<String>>,
    #[serde(default)]
    pub profiles: SerdeProfiles,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Empty {}
