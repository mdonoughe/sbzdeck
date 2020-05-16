#[cfg(not(any(target_arch = "x86")))]
compile_error!("This crate must be built for x86 for compatibility with sound drivers." +
    "(build for i686-pc-windows-msvc or suppress this error using feature ctsndcr_ignore_arch)");

mod sb;
mod settings;
mod types;

use crate::types::*;
use common::SerdeCardSettings;
use futures::channel::mpsc;
use futures::prelude::*;
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

const ACTION_SELECT_OUTPUT: &str = "io.github.mdonoughe.sbzdeck.selectoutput";

async fn connect(
    params: &RegistrationParams,
) -> Result<StreamDeckSocket<SerdeCardSettings, Empty, FromInspector, ToInspector>, ConnectError> {
    StreamDeckSocket::<SerdeCardSettings, Empty, FromInspector, ToInspector>::connect(
        params.port,
        params.event.to_string(),
        params.uuid.to_string(),
    )
    .await
}

async fn handle_new_action(
    logger: &Logger,
    state: &State,
    context: &str,
    action_state: Option<u8>,
) {
    let output = {
        let mut state = state.lock().unwrap();
        state.contexts.insert(context.to_owned());
        state.output
    };
    match output {
        Some(output) if Some(Into::<u8>::into(output)) != action_state => {
            debug!(logger, "Correcting state to {:?}", output);
            let mut state = state.lock().unwrap();
            state
                .out
                .send(MessageOut::SetState {
                    context: context.to_owned(),
                    payload: StatePayload {
                        state: output.into(),
                    },
                })
                .await
                .expect("failed to queue message");
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

async fn handle_press(
    logger: &Logger,
    state: &State,
    context: &str,
    payload: &KeyPayload<Empty>,
    trigger_save: &mut mpsc::Sender<()>,
) {
    let desired_state = payload
        .user_desired_state
        .unwrap_or_else(|| (payload.state.unwrap_or(0) + 1) % 2);
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
                // this should only happen with multiactions
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
            state
                .out
                .send(MessageOut::ShowOk {
                    context: context.to_string(),
                })
                .await
                .expect("failed to queue message");
        }
        Err(error) => {
            error!(
                logger,
                "Failed to set output to {}: {:?}", desired_state, error
            );
            state
                .out
                .send(MessageOut::ShowAlert {
                    context: context.to_string(),
                })
                .await
                .expect("failed to queue message")
        }
    }
}

async fn handle_message(
    logger: &Logger,
    message: Message<SerdeCardSettings, Empty, FromInspector>,
    state: &State,
    trigger_save: &mut mpsc::Sender<()>,
) {
    match message {
        Message::WillAppear {
            ref action,
            ref context,
            ref payload,
            ..
        } if action == ACTION_SELECT_OUTPUT => {
            handle_new_action(logger, state, &context, payload.state).await
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
            handle_press(logger, state, &context, &payload, trigger_save).await
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
                let mut state = state.lock().unwrap();
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
                state
                    .out
                    .send(MessageOut::SendToPropertyInspector {
                        action,
                        context,
                        payload: ToInspector::SetFeatures {
                            selected_parameters: response,
                        },
                    })
                    .await
                    .expect("failed to queue message");
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
}

#[tokio::main(max_threads=1)]
async fn main() {
    let params = &RegistrationParams::from_args(env::args()).unwrap();

    let (mut out_sink, mut out_stream) = mpsc::channel(2);
    let mut state = RawState {
        output: None,
        contexts: BTreeSet::new(),
        out: out_sink.clone(),
        settings: CardSettings::default(),
    };

    let (log_sink, mut log_stream) = mpsc::unbounded();
    let mut log_out_sink = out_sink.clone();
    let log_task = async {
        while let Some(evt) = log_stream.next().await {
            log_out_sink.send(evt).await.expect("failed to forward log")
        }
    };

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
    let save_context = params.uuid.clone();
    let save = async {
        let mut triggers = tokio::time::throttle(Duration::from_secs(5), save_trigger);
        while let Some(_) = triggers.next().await {
            debug!(save_log, "saving…");
            let settings = { settings::prepare_for_save(&state_save.lock().unwrap().settings) };
            match out_sink
                .send(MessageOut::SetGlobalSettings {
                    context: save_context.to_string(),
                    payload: settings,
                })
                .await
            {
                Ok(_) => debug!(save_log, "settings saved"),
                Err(error) => error!(save_log, "settings could not be saved: {:?}", error),
            }
        }
    };

    let state_events = state.clone();
    let logger_events = logger.clone();
    let mut trigger_save_events = trigger_save.clone();
    let events = async {
        let mut events = sb::watch(&logger).await.unwrap();
        while let Some(evt) = events.next().await {
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
                            let RawState {
                                ref mut out,
                                ref contexts,
                                ..
                            } = *state;
                            for context in contexts.iter() {
                                out.send(MessageOut::SetState {
                                    context: context.to_owned(),
                                    payload: StatePayload {
                                        state: output.into(),
                                    },
                                })
                                .await
                                .expect("failed to queue message");
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
        }
    };

    let get_settings_context = params.uuid.clone();
    let test = async {
        let (mut sink, mut stream) = connect(params).await.expect("connection failed").split();

        let send_task = async {
            sink.send(MessageOut::GetGlobalSettings {
                context: get_settings_context,
            })
            .await
            .expect("failed to send message");
            while let Some(message) = out_stream.next().await {
                sink.send(message).await.expect("failed to send message");
            }
        };

        let receive_task = async {
            let logger_e = logger.clone();
            while let Some(message) = stream.next().await {
                match message {
                    Err(e) => crit!(logger_e, "receive failed {:?}", e),
                    Ok(message) => {
                        debug!(logger, "received {:?}", message);
                        handle_message(&logger, message, &state, &mut trigger_save).await;
                    }
                }
            }
        };

        futures::join!(send_task, receive_task);
    };

    futures::join!(save, events, log_task, test);
}
