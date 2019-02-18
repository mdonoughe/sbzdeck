use crate::types::*;
use indexmap::IndexMap;
use sbz_switch::soundcore::SoundCoreParamValue;
use serde_derive::{Deserialize, Serialize};
use std::env;
use std::fs::File;

#[derive(Default, Deserialize, Serialize)]
pub struct SerdeProfile {
    pub volume: Option<f32>,
    #[serde(default)]
    pub parameters: serde_json::Map<String, serde_json::Value>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct SerdeProfiles {
    #[serde(default)]
    pub headphones: SerdeProfile,
    #[serde(default)]
    pub speakers: SerdeProfile,
}

#[derive(Default, Deserialize, Serialize)]
pub struct SerdeCardSettings {
    #[serde(default)]
    pub selected_parameters: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub profiles: SerdeProfiles,
}

fn convert_to_soundcore(
    value: serde_json::Map<String, serde_json::Value>,
) -> IndexMap<String, IndexMap<String, SoundCoreParamValue>> {
    value
        .into_iter()
        .filter_map(|(name, params)| match params {
            serde_json::Value::Object(params) => Some((
                name,
                params
                    .into_iter()
                    .filter_map(|(name, value)| match value {
                        serde_json::Value::Number(n) => match n.as_i64() {
                            Some(n) if n < i64::from(i32::min_value()) => None,
                            Some(n) if n <= i64::from(i32::max_value()) => {
                                Some((name, SoundCoreParamValue::I32(n as i32)))
                            }
                            Some(n) if n <= i64::from(u32::max_value()) => {
                                Some((name, SoundCoreParamValue::U32(n as u32)))
                            }
                            Some(_) => None,
                            None => {
                                Some((name, SoundCoreParamValue::Float(n.as_f64().unwrap() as f32)))
                            }
                        },
                        serde_json::Value::Bool(b) => Some((name, SoundCoreParamValue::Bool(b))),
                        _ => None,
                    })
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

pub fn load() -> Result<CardSettings, serde_json::Error> {
    let mut path = env::current_exe().unwrap_or_default();
    path.pop();
    path.push("sbzdeck.json");
    let file = File::open(path).map_err(serde_json::Error::io)?;
    let de: SerdeCardSettings = serde_json::from_reader(file)?;
    Ok(CardSettings {
        selected_parameters: de
            .selected_parameters
            .into_iter()
            .filter_map(|(name, params)| match params {
                serde_json::Value::Array(params) => Some((
                    name,
                    params
                        .into_iter()
                        .filter_map(|param| match param {
                            serde_json::Value::String(param) => Some(param),
                            _ => None,
                        })
                        .collect(),
                )),
                _ => None,
            })
            .collect(),
        profiles: Profiles {
            headphones: Profile {
                volume: de.profiles.headphones.volume,
                parameters: convert_to_soundcore(de.profiles.headphones.parameters),
            },
            speakers: Profile {
                volume: de.profiles.speakers.volume,
                parameters: convert_to_soundcore(de.profiles.speakers.parameters),
            },
        },
    })
}

fn convert_from_soundcore(
    value: &IndexMap<String, IndexMap<String, SoundCoreParamValue>>,
) -> serde_json::Map<String, serde_json::Value> {
    value
        .into_iter()
        .map(|(name, params)| {
            (
                name.to_string(),
                serde_json::Value::Object(
                    params
                        .into_iter()
                        .filter_map(|(name, value)| match value {
                            SoundCoreParamValue::I32(n) => {
                                Some((name.to_string(), serde_json::Value::Number((*n).into())))
                            }
                            SoundCoreParamValue::U32(n) => {
                                Some((name.to_string(), serde_json::Value::Number((*n).into())))
                            }
                            SoundCoreParamValue::Float(n) => {
                                serde_json::Number::from_f64((*n).into())
                                    .map(|v| (name.to_string(), serde_json::Value::Number(v)))
                            }
                            SoundCoreParamValue::Bool(b) => {
                                Some((name.to_string(), serde_json::Value::Bool(*b)))
                            }
                            _ => None,
                        })
                        .collect(),
                ),
            )
        })
        .collect()
}

pub fn prepare_for_save(settings: &CardSettings) -> SerdeCardSettings {
    SerdeCardSettings {
        selected_parameters: settings
            .selected_parameters
            .iter()
            .map(|(name, params)| {
                (
                    name.to_string(),
                    serde_json::Value::Array(
                        params
                            .iter()
                            .map(|param| serde_json::Value::String(param.to_string()))
                            .collect(),
                    ),
                )
            })
            .collect(),
        profiles: SerdeProfiles {
            headphones: SerdeProfile {
                volume: settings.profiles.headphones.volume,
                parameters: convert_from_soundcore(&settings.profiles.headphones.parameters),
            },
            speakers: SerdeProfile {
                volume: settings.profiles.speakers.volume,
                parameters: convert_from_soundcore(&settings.profiles.speakers.parameters),
            },
        },
    }
}

pub fn save(settings: &SerdeCardSettings) -> Result<(), serde_json::Error> {
    let mut path = env::current_exe().unwrap_or_default();
    path.pop();
    path.push("sbzdeck.json");
    let file = File::create(path).map_err(serde_json::Error::io)?;
    serde_json::to_writer_pretty(file, settings)
}
