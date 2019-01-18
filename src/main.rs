#[cfg(not(any(target_arch = "x86")))]
compile_error!("This crate must be built for x86 for compatibility with sound drivers." +
    "(build for i686-pc-windows-msvc or suppress this error using feature ctsndcr_ignore_arch)");

extern crate indexmap;
extern crate sbz_switch;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate slog;
extern crate sloggers;
extern crate streamdeck_rs;
extern crate tokio;

mod logger;
mod sb;
mod settings;
mod types;

use crate::types::*;
use futures::prelude::*;
use futures::sync::mpsc;
use indexmap::IndexMap;
use sbz_switch::soundcore::SoundCoreParamValue;
use sbz_switch::{Configuration, EndpointConfiguration};
use slog::{crit, debug, error, info, warn, Logger};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::{env, iter};
use streamdeck_rs::registration::RegistrationParams;
use streamdeck_rs::socket::{ConnectError, StreamDeckSocket};
use streamdeck_rs::{KeyPayload, Message, MessageOut, StatePayload};

const ACTION_SELECT_OUTPUT: &str = "io.github.mdonoughe.sbzdeck.selectOutput";

fn connect() -> impl Future<Item = StreamDeckSocket<Empty, Empty, Empty>, Error = ConnectError> {
    let params = RegistrationParams::from_args(env::args()).unwrap();
    StreamDeckSocket::<Empty, Empty, Empty>::connect(params.port, params.event, params.uuid)
}

fn handle_new_action(logger: &Logger, state: &State, context: &str, action_state: u8) {
    let output = {
        let mut state = state.lock().unwrap();
        state.contexts.insert(context.to_owned());
        state.output
    };
    match output {
        Some(output) if Into::<u8>::into(output) != action_state => {
            debug!(logger, "Correcting state to {:?}", output);
            let logger_e = logger.clone();
            let state = state.lock().unwrap();
            tokio::spawn(
                state
                    .out
                    .clone()
                    .send(MessageOut::SetState {
                        context: context.to_owned(),
                        payload: StatePayload {
                            state: output.into(),
                        },
                    })
                    .map_err(move |e| error!(logger_e, "failed to queue message: {:?}", e))
                    .map(|_| ()),
            );
        }
        Some(_) => {
            debug!(logger, "Current state matches button state");
        }
        None => {
            warn!(logger, "Current output is unknown");
        }
    }
}

fn handle_remove_action(state: &State, context: &str) {
    let mut state = state.lock().unwrap();
    state.contexts.remove(context);
}

fn handle_press(logger: &Logger, state: &State, context: &str, payload: &KeyPayload<Empty>) {
    let desired_state = payload
        .user_desired_state
        .unwrap_or_else(|| (payload.state + 1) % 2);
    let output = match desired_state {
        0 => Output::Headphones,
        1 => Output::Speakers,
        _ => unreachable!(),
    };

    let mut state = state.lock().unwrap();
    // save back current state
    //TODO: save to disk
    //TODO: remove after implementing monitoring?
    match sb::get_current_profile(&logger) {
        Ok(Some((current_device_output, current_device_profile))) => {
            info!(
                logger,
                "detected current output to be {:?}", current_device_output
            );
            if output == current_device_output {
                // this should only happen with multiactions after implementing monitoring
                return;
            }
            state.profiles[current_device_output] = current_device_profile;
        }
        Ok(None) => {
            error!(
                logger,
                "could not find output device in sound card configuration"
            );
        }
        Err(error) => error!(
            logger,
            "error reading sound card configuration: {:?}", error
        ),
    }

    let profile = &state.profiles[output];

    let mut creative: IndexMap<String, IndexMap<String, SoundCoreParamValue>> = iter::once((
        "Device Control".to_owned(),
        iter::once((
            "SelectOutput".to_owned(),
            SoundCoreParamValue::U32(u32::from(desired_state)),
        ))
        .collect(),
    ))
    .collect();

    for (name, feature) in state.selected_parameters.iter() {
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
    let result = sbz_switch::set(&logger, None, &configuration, true);
    match result {
        Ok(_) => {
            state.output = Some(output);
            debug!(logger, "Set output to {}", desired_state);
            let logger_e = logger.clone();
            tokio::spawn(
                state
                    .out
                    .clone()
                    .send(MessageOut::ShowOk {
                        context: context.to_string(),
                    })
                    .map_err(move |e| error!(logger_e, "failed to queue message: {:?}", e))
                    .map(|_| ()),
            );
        }
        Err(error) => {
            error!(
                logger,
                "Failed to set output to {}: {:?}", desired_state, error
            );
            let logger_e = logger.clone();
            tokio::spawn(
                state
                    .out
                    .clone()
                    .send(MessageOut::ShowAlert {
                        context: context.to_string(),
                    })
                    .map_err(move |e| error!(logger_e, "failed to queue message: {:?}", e))
                    .map(|_| ()),
            );
        }
    }
}

fn handle_message(
    logger: &Logger,
    message: &Message<Empty, Empty>,
    state: &State,
) -> Result<(), ()> {
    match &message {
        Message::WillAppear {
            action,
            context,
            payload,
            ..
        } if action == ACTION_SELECT_OUTPUT => {
            handle_new_action(logger, state, context, payload.state)
        }
        Message::WillDisappear {
            action, context, ..
        } if action == ACTION_SELECT_OUTPUT => handle_remove_action(state, context),
        Message::KeyUp {
            action,
            context,
            payload,
            ..
        } if action == ACTION_SELECT_OUTPUT => handle_press(logger, state, context, payload),
        _ => {}
    }
    Ok(())
}

fn main() {
    let logger = logger::create();
    info!(logger, "launched {:?}", env::args().collect::<Vec<_>>());

    let settings = match settings::load() {
        Ok(settings) => settings,
        Err(error) => {
            if error.is_io() {
                let error = std::io::Error::from(error);
                match error.kind() {
                    std::io::ErrorKind::NotFound => {
                        warn!(logger, "settings file was not found");
                    }
                    _ => {
                        error!(logger, "failed to load settings: {:?}", error);
                    }
                }
            } else {
                error!(logger, "failed to load settings: {:?}", error);
            }
            CardSettings::default()
        }
    };
    debug!(logger, "settings: {:?}", settings);

    let (out_sink, out_stream) = mpsc::channel(1);
    let mut state = RawState {
        output: None,
        selected_parameters: settings.selected_parameters,
        contexts: BTreeSet::new(),
        out: out_sink,
        profiles: Profiles {
            headphones: Profile {
                volume: settings.profiles.headphones.volume,
                parameters: settings.profiles.headphones.parameters,
            },
            speakers: Profile {
                volume: settings.profiles.speakers.volume,
                parameters: settings.profiles.speakers.parameters,
            },
        },
    };

    match sb::get_current_profile(&logger) {
        Ok(Some((output, profile))) => {
            info!(logger, "detected current output to be {:?}", output);
            state.output = Some(output);
            state.profiles[output] = profile;
        }
        Ok(None) => {
            error!(
                logger,
                "could not find output device in sound card configuration"
            );
        }
        Err(error) => error!(
            logger,
            "error reading sound card configuration: {:?}", error
        ),
    }

    let state = Arc::new(Mutex::new(state));

    let logger_e = logger.clone();
    let test = connect()
        .map_err(move |e| crit!(logger_e, "connection failed {:?}", e))
        .and_then(move |s| {
            info!(logger, "connected!");
            let (sink, stream) = s.split();

            let logger_e = logger.clone();
            tokio::spawn(
                sink.send_all(out_stream.map_err(|_| unreachable!()))
                    .map_err(move |e| error!(logger_e, "failed to send message: {:?}", e))
                    .map(|_| ()),
            );

            let logger_e = logger.clone();
            stream
                .map_err(move |e| crit!(logger_e, "receive failed {:?}", e))
                .for_each(move |message| {
                    debug!(logger, "received {:?}", message);
                    handle_message(&logger, &message, &state)
                })
        });
    //TODO: monitor for changes
    tokio::run(test.map_err(|e| panic!("{:?}", e)));
}
