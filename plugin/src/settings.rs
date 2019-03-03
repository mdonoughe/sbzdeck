use crate::types::*;
use common::{SerdeCardSettings, SerdeProfile, SerdeProfiles};
use indexmap::IndexMap;
use sbz_switch::soundcore::SoundCoreParamValue;
use std::collections::BTreeMap;

fn convert_to_soundcore(
    value: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
) -> IndexMap<String, IndexMap<String, SoundCoreParamValue>> {
    value
        .into_iter()
        .map(|(name, params)| {
            (
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
            )
        })
        .collect()
}

pub fn load(de: SerdeCardSettings) -> Result<CardSettings, serde_json::Error> {
    Ok(CardSettings {
        selected_parameters: de.selected_parameters,
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
) -> BTreeMap<String, BTreeMap<String, serde_json::Value>> {
    value
        .into_iter()
        .map(|(name, params)| {
            (
                name.to_string(),
                params
                    .into_iter()
                    .filter_map(|(name, value)| match value {
                        SoundCoreParamValue::I32(n) => {
                            Some((name.to_string(), serde_json::Value::Number((*n).into())))
                        }
                        SoundCoreParamValue::U32(n) => {
                            Some((name.to_string(), serde_json::Value::Number((*n).into())))
                        }
                        SoundCoreParamValue::Float(n) => serde_json::Number::from_f64((*n).into())
                            .map(|v| (name.to_string(), serde_json::Value::Number(v))),
                        SoundCoreParamValue::Bool(b) => {
                            Some((name.to_string(), serde_json::Value::Bool(*b)))
                        }
                        _ => None,
                    })
                    .collect(),
            )
        })
        .collect()
}

pub fn prepare_for_save(settings: &CardSettings) -> SerdeCardSettings {
    SerdeCardSettings {
        selected_parameters: settings.selected_parameters.clone(),
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
