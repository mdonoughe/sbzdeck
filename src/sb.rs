use crate::types::*;
use futures::prelude::*;
use futures::sync::{mpsc, oneshot};
use indexmap::{IndexMap, IndexSet};
use sbz_switch::soundcore::{SoundCoreEvent, SoundCoreParamValue};
use sbz_switch::{Configuration, EndpointConfiguration, Win32Error};
use slog::error;
use slog::Logger;
use std::{iter, thread};

pub fn get_current_profile(
    logger: &Logger,
) -> Result<Option<(Output, Profile)>, Box<dyn std::error::Error>> {
    match sbz_switch::dump(logger, None) {
        Ok(device_state) => Ok(match device_state.creative {
            Some(creative) => {
                let output = creative
                    .get("Device Control")
                    .and_then(|control| control.get("SelectOutput").and_then(Output::try_from));
                match output {
                    Some(output) => Some((
                        output,
                        Profile {
                            volume: device_state
                                .endpoint
                                .as_ref()
                                .and_then(|endpoint| endpoint.volume),
                            parameters: creative,
                        },
                    )),
                    None => None,
                }
            }
            None => None,
        }),
        Err(error) => Err(error),
    }
}

pub fn apply_profile(
    logger: &Logger,
    output: Output,
    profile: &Profile,
    selected_parameters: &IndexMap<String, IndexSet<String>>,
) -> Result<(), Box<std::error::Error>> {
    let mut creative: IndexMap<String, IndexMap<String, SoundCoreParamValue>> = iter::once((
        "Device Control".to_owned(),
        iter::once((
            "SelectOutput".to_owned(),
            SoundCoreParamValue::U32(u32::from(Into::<u8>::into(output))),
        ))
        .collect(),
    ))
    .collect();

    for (name, feature) in selected_parameters.iter() {
        if let Some(feature_in) = profile.parameters.get(name) {
            let feature_out = creative.entry(name.to_owned()).or_default();
            for name in feature {
                if let Some(value) = feature_in.get(name) {
                    feature_out.insert(name.to_owned(), value.clone());
                }
            }
        }
    }

    let configuration = Configuration {
        endpoint: Some(EndpointConfiguration {
            volume: profile.volume,
        }),
        creative: Some(creative),
    };
    sbz_switch::set(&logger, None, &configuration, true)
}

#[derive(Debug)]
pub struct ChangeEvent {
    pub feature: String,
    pub parameter: String,
    pub value: SoundCoreParamValue,
}

pub fn watch(logger: &Logger) -> Result<mpsc::Receiver<Result<ChangeEvent, Win32Error>>, ()> {
    let (start_tx, start_rx) = oneshot::channel();
    let logger = logger.clone();
    thread::Builder::new()
        .name("event thread".into())
        .spawn(move || match sbz_switch::watch(&logger, None) {
            Ok(iterator) => {
                let (event_tx, event_rx) = mpsc::channel(64);
                start_tx.send(Ok(event_rx)).unwrap();
                let mut event_tx = event_tx.wait();
                for event in iterator {
                    match event {
                        Ok(SoundCoreEvent::ParamChange { feature, parameter }) => {
                            let event = match parameter.get() {
                                Ok(value) => Ok(ChangeEvent {
                                    feature: feature.description.to_owned(),
                                    parameter: parameter.description.to_owned(),
                                    value,
                                }),
                                Err(error) => Err(error),
                            };
                            event_tx.send(event).unwrap();
                        }
                        Ok(_) => {}
                        Err(error) => event_tx.send(Err(error)).unwrap(),
                    }
                }
            }
            Err(error) => {
                error!(logger, "failed to listen for events: {:?}", error);
                start_tx.send(Err(())).unwrap();
            }
        })
        .unwrap();
    start_rx.wait().unwrap()
}
