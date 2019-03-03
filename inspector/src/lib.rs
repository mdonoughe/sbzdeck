extern crate indexmap;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate stdweb;
#[macro_use]
extern crate yew;

mod feature;
mod message;
mod parameter;
mod registration;
mod socket;

use feature::Feature;
use indexmap::IndexMap;
use socket::{StreamDeckSocketService, StreamDeckSocketTask};
use std::collections::BTreeSet;
use stdweb::js_export;
use stdweb::web::{document, INode};
use yew::prelude::*;
use yew::services::websocket::WebSocketStatus;

type Message = message::Message<common::SerdeCardSettings, common::Empty, common::ToInspector>;
type RegistrationParams = registration::InspectorRegistrationParams<common::Empty>;

#[js_export]
fn connect_elgato_stream_deck_socket(
    port: &str,
    property_inspector_uuid: String,
    register_event: String,
    info: &str,
    action_info: &str,
) {
    let url = format!("ws://localhost:{}", port);

    let info = serde_json::from_str(info).unwrap();
    let action_info = serde_json::from_str(action_info).unwrap();

    let params = RegistrationParams {
        url,
        property_inspector_uuid,
        register_event,
        info,
        action_info,
    };

    let body = document().body().unwrap();
    let target = document().create_element("div").unwrap();
    body.append_child(&target);

    yew::initialize();
    let mut scope = App::<Model>::new().mount(target);
    yew::run_loop();

    scope.send_message(ComponentMessage::Connect(params));
}

#[derive(Clone, Default, Eq, PartialEq)]
pub struct FeatureInfo {
    pub is_expanded: bool,
    pub parameters: IndexMap<String, bool>,
}

pub struct Model {
    link: ComponentLink<Model>,
    socket: StreamDeckSocketService<
        common::SerdeCardSettings,
        common::Empty,
        common::ToInspector,
        common::FromInspector,
    >,
    task: Option<
        StreamDeckSocketTask<common::SerdeCardSettings, common::Empty, common::FromInspector>,
    >,
    registration_params: Option<RegistrationParams>,
    selected_params: IndexMap<String, FeatureInfo>,
}

pub enum ComponentMessage {
    Connect(RegistrationParams),
    Message(Message),
    Status(WebSocketStatus),
    SetParameter {
        feature: String,
        parameter: String,
        is_selected: bool,
    },
    SetFeatureExpanded {
        feature: String,
        is_expanded: bool,
    },
}

impl Component for Model {
    type Message = ComponentMessage;
    type Properties = ();

    fn create(_properties: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            socket: StreamDeckSocketService::new(),
            task: None,
            selected_params: IndexMap::new(),
            registration_params: None,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        match message {
            ComponentMessage::Connect(message) => {
                self.task = Some(self.socket.connect(
                    &message.url,
                    self.link.send_back(ComponentMessage::Message),
                    self.link.send_back(ComponentMessage::Status),
                ));
                self.registration_params = Some(message);
                false
            }
            ComponentMessage::Message(message) => {
                match message {
                    message::Message::SendToPropertyInspector { payload, .. } => match payload {
                        common::ToInspector::SetFeatures {
                            selected_parameters,
                        } => {
                            let expanded = self
                                .selected_params
                                .iter()
                                .filter(|(_, f)| f.is_expanded)
                                .map(|(n, _)| n)
                                .collect::<BTreeSet<_>>();
                            self.selected_params = selected_parameters
                                .into_iter()
                                .map(|(name, params)| {
                                    let is_expanded = expanded.contains(&name);
                                    (
                                        name,
                                        FeatureInfo {
                                            is_expanded,
                                            parameters: params,
                                        },
                                    )
                                })
                                .collect()
                        }
                    },
                    _ => {}
                }
                true
            }
            ComponentMessage::Status(status) => {
                if let WebSocketStatus::Opened = status {
                    let task = self.task.as_mut().unwrap();
                    let registration_params = self.registration_params.as_ref().unwrap();
                    task.register(
                        &registration_params.register_event,
                        &registration_params.property_inspector_uuid,
                    );
                    task.send(&message::MessageOut::SendToPlugin {
                        action: registration_params.action_info.action.to_string(),
                        context: registration_params.property_inspector_uuid.to_string(),
                        payload: common::FromInspector::GetFeatures,
                    });
                }
                false
            }
            ComponentMessage::SetFeatureExpanded {
                feature,
                is_expanded,
            } => {
                let feature = self.selected_params.get_mut(&feature).unwrap();
                if feature.is_expanded == is_expanded {
                    false
                } else {
                    feature.is_expanded = is_expanded;
                    true
                }
            }
            ComponentMessage::SetParameter {
                feature,
                parameter,
                is_selected,
            } => {
                let changed = {
                    let feature = self.selected_params.get_mut(&feature).unwrap();
                    let parameter = feature.parameters.get_mut(&parameter).unwrap();
                    if *parameter == is_selected {
                        false
                    } else {
                        *parameter = is_selected;
                        true
                    }
                };
                if changed {
                    let task = self.task.as_mut().unwrap();
                    let registration_params = self.registration_params.as_ref().unwrap();
                    task.send(&message::MessageOut::SendToPlugin {
                        action: registration_params.action_info.action.to_string(),
                        context: registration_params.property_inspector_uuid.to_string(),
                        payload: common::FromInspector::SetFeatures {
                            selected_parameters: self
                                .selected_params
                                .iter()
                                .filter(|(_, info)| {
                                    info.parameters.iter().any(|(_, is_selected)| *is_selected)
                                })
                                .map(|(name, info)| {
                                    (
                                        name.to_string(),
                                        info.parameters
                                            .iter()
                                            .filter(|(_, is_selected)| **is_selected)
                                            .map(|(name, _)| name.to_string())
                                            .collect(),
                                    )
                                })
                                .collect(),
                        },
                    });
                }
                changed
            }
        }
    }
}

impl Renderable<Model> for Model {
    fn view(&self) -> Html<Self> {
        html! {
            <div>
                { for self.selected_params.iter().map(|(name, info)| {
                    let cb1_name = name.to_string();
                    let cb2_name = name.to_string();
                    html! {
                        <Feature: name=name,
                            is_expanded=info.is_expanded,
                            parameters=&info.parameters,
                            onchange=move |(parameter, is_selected)| { ComponentMessage::SetParameter {
                                feature: cb1_name.clone(),
                                parameter,
                                is_selected
                            } },
                            onexpandchange=move |is_expanded| { ComponentMessage::SetFeatureExpanded {
                                feature: cb2_name.clone(),
                                is_expanded
                            } }, />
                    }
                }) }
            </div>
        }
    }
}
