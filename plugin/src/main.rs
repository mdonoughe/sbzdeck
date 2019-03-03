#[cfg(not(any(target_arch = "x86")))]
compile_error!("This crate must be built for x86 for compatibility with sound drivers." +
    "(build for i686-pc-windows-msvc or suppress this error using feature ctsndcr_ignore_arch)");

extern crate common;
extern crate indexmap;
extern crate sbz_switch;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate slog;
extern crate streamdeck_rs;
extern crate tokio;

mod sb;
mod settings;
mod types;

use crate::types::*;
use common::SerdeCardSettings;
use futures::prelude::*;
use futures::sync::mpsc;
use sb::ChangeEvent;
use slog::{crit, debug, error, info, o, warn, Drain, Logger};
use std::collections::BTreeSet;
use std::env;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use streamdeck_rs::logging::StreamDeckDrain;
use streamdeck_rs::registration::RegistrationParams;
use streamdeck_rs::socket::{ConnectError, StreamDeckSocket};
use streamdeck_rs::{KeyPayload, Message, MessageOut, StatePayload};
use tokio::prelude::*;

const ACTION_SELECT_OUTPUT: &str = "io.github.mdonoughe.sbzdeck.selectOutput";

fn connect(
    params: &RegistrationParams,
) -> impl Future<
    Item = StreamDeckSocket<SerdeCardSettings, Empty, FromInspector, ToInspector>,
    Error = ConnectError,
> {
    StreamDeckSocket::<SerdeCardSettings, Empty, FromInspector, ToInspector>::connect(
        params.port,
        params.event.to_string(),
        params.uuid.to_string(),
    )
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

fn handle_press(
    logger: &Logger,
    state: &State,
    context: &str,
    payload: &KeyPayload<Empty>,
    trigger_save: &mut mpsc::Sender<()>,
) {
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
    // Why update the state right before switching even if events are being
    // monitored? Changes to the device state are not atomic, so if the user
    // manually switches from headphones to speakers, we don't want to
    // immediately overwrite all the speaker settings with the headphone
    // settings. Therefore, only *changed* settings are saved into our active
    // profile. However, the purpose of this plugin is to remember audio
    // settings when switching outputs, so it seems wrong if the user can switch
    // to speakers manually, toggle twice, and not have the same settings as
    // before toggling. This means pressing the toggle key basically acts as
    // confirmation that the current settings are desired settings in the case
    // where we are not sure.
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
            state.settings.profiles[current_device_output] = current_device_profile;
            let _ = trigger_save.try_send(());
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

    match sb::apply_profile(
        logger,
        output,
        &state.settings.profiles[output],
        &state.settings.selected_parameters,
    ) {
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
    message: Message<SerdeCardSettings, Empty, FromInspector>,
    state: &State,
    trigger_save: &mut mpsc::Sender<()>,
) -> Result<(), ()> {
    match message {
        Message::WillAppear {
            ref action,
            ref context,
            ref payload,
            ..
        } if action == ACTION_SELECT_OUTPUT => {
            handle_new_action(logger, state, &context, payload.state)
        }
        Message::WillDisappear {
            ref action,
            ref context,
            ..
        } if action == ACTION_SELECT_OUTPUT => handle_remove_action(state, &context),
        Message::KeyUp {
            ref action,
            ref context,
            ref payload,
            ..
        } if action == ACTION_SELECT_OUTPUT => {
            handle_press(logger, state, &context, &payload, trigger_save)
        }
        Message::DidReceiveGlobalSettings { payload, .. } => {
            match settings::load(payload.settings) {
                Ok(settings) => {
                    let mut state = state.lock().unwrap();
                    state.settings = settings;
                    info!(logger, "loaded settings");
                }
                Err(error) => {
                    error!(logger, "error loading settings: {:?}", error);
                }
            }
        }
        Message::SendToPlugin {
            action,
            context,
            payload,
            ..
        } => match payload {
            FromInspector::GetFeatures => {
                let available = sbz_switch::dump(&logger, None)
                    .ok()
                    .and_then(|s| s.creative)
                    .unwrap_or_default();
                let state = state.lock().unwrap();
                let response = available
                    .into_iter()
                    .map(|(k, v)| {
                        let feature_selection = state.settings.selected_parameters.get(&k);
                        (
                            k,
                            v.into_iter()
                                .map(|(k, _)| {
                                    let selected = feature_selection
                                        .map(|s| s.contains(&k))
                                        .unwrap_or_default();
                                    (k, selected)
                                })
                                .collect(),
                        )
                    })
                    .collect();
                let logger_e = logger.clone();
                tokio::spawn(
                    state
                        .out
                        .clone()
                        .send(MessageOut::SendToPropertyInspector {
                            action: action,
                            context: context,
                            payload: ToInspector::SetFeatures {
                                selected_parameters: response,
                            },
                        })
                        .map_err(move |e| error!(logger_e, "failed to queue message: {:?}", e))
                        .map(|_| ()),
                );
            }
            FromInspector::SetFeatures {
                selected_parameters,
            } => {
                let available = sbz_switch::dump(&logger, None)
                    .ok()
                    .and_then(|s| s.creative)
                    .unwrap_or_default();
                let mut state = state.lock().unwrap();
                state.settings.selected_parameters = available
                    .into_iter()
                    .filter_map(|(k, v)| {
                        selected_parameters.get(&k).map(|feature_selection| {
                            (
                                k,
                                v.into_iter()
                                    .filter(|(k, _)| feature_selection.contains(k))
                                    .map(|(k, _)| k)
                                    .collect(),
                            )
                        })
                    })
                    .collect();
                info!(
                    logger,
                    "selecting features are now {:?}", state.settings.selected_parameters
                );
                let _ = trigger_save.try_send(());
            }
        },
        _ => {}
    }
    Ok(())
}

fn main() {
    let params = RegistrationParams::from_args(env::args()).unwrap();

    let (out_sink, out_stream) = mpsc::channel(1);
    let mut state = RawState {
        output: None,
        contexts: BTreeSet::new(),
        out: out_sink.clone(),
        settings: CardSettings::default(),
    };

    let (log_sink, log_stream) = mpsc::unbounded();
    let log_task = log_stream.forward(
        out_sink
            .clone()
            .sink_map_err(|error| panic!("failed to forward log: {:?}", error)),
    );

    let logger = slog::Logger::root(StreamDeckDrain::new(log_sink).fuse(), o!());

    match sb::get_current_profile(&logger) {
        Ok(Some((output, profile))) => {
            info!(logger, "detected current output to be {:?}", output);
            state.output = Some(output);
            state.settings.profiles[output] = profile;
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

    let (mut trigger_save, save_trigger) = mpsc::channel(0);
    let state_save = state.clone();
    let save_log = logger.clone();
    let save_sink = out_sink.clone();
    let save_context = params.uuid.clone();
    let save = save_trigger
        .throttle(Duration::from_secs(5))
        .for_each(move |_| {
            debug!(save_log, "saving…");
            let settings = { settings::prepare_for_save(&state_save.lock().unwrap().settings) };
            let log = save_log.clone();
            save_sink
                .clone()
                .send(MessageOut::SetGlobalSettings {
                    context: save_context.to_string(),
                    payload: settings,
                })
                .then(move |r| {
                    match r {
                        Ok(_) => debug!(log, "settings saved"),
                        Err(error) => error!(log, "settings could not be saved: {:?}", error),
                    }
                    Ok(())
                })
        });

    let state_events = state.clone();
    let logger_events = logger.clone();
    let mut trigger_save_events = trigger_save.clone();
    let events = sb::watch(&logger).unwrap().for_each(move |evt| {
        let evt = evt.unwrap();
        debug!(logger_events, "saw change: {:?}", evt);
        let mut state = state_events.lock().unwrap();
        match evt {
            ChangeEvent::SoundCore(ref evt)
                if evt.feature == "Device Control" && evt.parameter == "SelectOutput" =>
            {
                match Output::try_from(&evt.value) {
                    Some(output) => {
                        state.output = Some(output);
                        for context in state.contexts.iter() {
                            let logger_e = logger_events.clone();
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
                                    .map_err(move |e| {
                                        error!(logger_e, "failed to queue message: {:?}", e)
                                    })
                                    .map(|_| ()),
                            );
                        }
                    }
                    None => {
                        warn!(
                            logger_events,
                            "output device changed to unrecognized value {:?}", evt.value
                        );
                        state.output = None;
                    }
                }
            }
            ChangeEvent::SoundCore(evt) => {
                if let Some(output) = state.output {
                    // Why update the profile here if we update the profile again right
                    // before switching? If the user changes a setting and then
                    // manually switches outputs, we want to capture that setting for
                    // the next time the user switches back to the original output.
                    let feature = state.settings.profiles[output]
                        .parameters
                        .entry(evt.feature)
                        .or_default();
                    feature.insert(evt.parameter, evt.value);
                }
            }
            ChangeEvent::Volume(volume) => {
                if let Some(output) = state.output {
                    state.settings.profiles[output].volume = Some(volume);
                }
            }
        }
        let _ = trigger_save_events.try_send(());
        Ok(())
    });

    let get_settings_context = params.uuid.clone();
    let test = connect(&params)
        .map_err(move |e| panic!("connection failed {:?}", e))
        .and_then(move |s| {
            info!(logger, "connected!");
            let (sink, stream) = s.split();

            let logger_e = logger.clone();
            tokio::spawn(
                stream::once(Ok(MessageOut::GetGlobalSettings {
                    context: get_settings_context,
                }))
                .chain(out_stream)
                .forward(
                    sink.sink_map_err(move |e| error!(logger_e, "failed to send message: {:?}", e)),
                )
                .map(|_| ()),
            );

            let logger_e = logger.clone();
            stream
                .map_err(move |e| crit!(logger_e, "receive failed {:?}", e))
                .for_each(move |message| {
                    debug!(logger, "received {:?}", message);
                    handle_message(&logger, message, &state, &mut trigger_save)
                })
        });
    tokio::run(
        Future::select(
            Future::select(save.map_err(|e| panic!("{:?}", e)), events)
                .map(|_| ())
                .map_err(|_| ()),
            Future::select(
                log_task.map(|_| ()),
                test.map(|_| ()).map_err(|e| panic!("{:?}", e)),
            )
            .map(|_| ())
            .map_err(|_| ()),
        )
        .map(|_| ())
        .map_err(|_| ()),
    );
}
