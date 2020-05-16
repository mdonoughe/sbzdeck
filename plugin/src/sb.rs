use crate::types::*;
use futures::channel::{mpsc, oneshot};
use futures::executor;
use futures::prelude::*;
use indexmap::{IndexMap, IndexSet};
use sbz_switch::media::VolumeNotification;
use sbz_switch::soundcore::{SoundCoreEvent, SoundCoreParamValue};
use sbz_switch::{Configuration, EndpointConfiguration, SoundCoreOrVolumeEvent, Win32Error};
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
) -> Result<(), Box<dyn std::error::Error>> {
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
pub enum ChangeEvent {
    SoundCore(SoundCoreChangeEvent),
    Volume(f32),
}

#[derive(Debug)]
pub struct SoundCoreChangeEvent {
    pub feature: String,
    pub parameter: String,
    pub value: SoundCoreParamValue,
}

pub async fn watch(logger: &Logger) -> Result<mpsc::Receiver<Result<ChangeEvent, Win32Error>>, ()> {
    let (start_tx, start_rx) = oneshot::channel();
    let logger = logger.clone();
    thread::Builder::new()
        .name("event thread".into())
        .spawn(move || match sbz_switch::watch_with_volume(&logger, None) {
            Ok(iterator) => {
                let (mut event_tx, event_rx) = mpsc::channel(64);
                start_tx.send(Ok(event_rx)).unwrap();
                for event in iterator {
                    match event {
                        Ok(SoundCoreOrVolumeEvent::SoundCore(SoundCoreEvent::ParamChange {
                            feature,
                            parameter,
                        })) => {
                            let event = match parameter.get() {
                                Ok(value) => Ok(ChangeEvent::SoundCore(SoundCoreChangeEvent {
                                    feature: feature.description.to_owned(),
                                    parameter: parameter.description.to_owned(),
                                    value,
                                })),
                                Err(error) => Err(error),
                            };
                            executor::block_on(event_tx.send(event)).unwrap();
                        }
                        Ok(SoundCoreOrVolumeEvent::Volume(VolumeNotification {
                            volume,
                            is_muted,
                            ..
                        })) if !is_muted => {
                            executor::block_on(event_tx.send(Ok(ChangeEvent::Volume(volume))))
                                .unwrap()
                        }
                        Ok(_) => {}
                        Err(error) => executor::block_on(event_tx.send(Err(error))).unwrap(),
                    }
                }
            }
            Err(error) => {
                error!(logger, "failed to listen for events: {:?}", error);
                start_tx.send(Err(())).unwrap();
            }
        })
        .unwrap();
    start_rx.await.unwrap()
}
