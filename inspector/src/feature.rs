use super::parameter::Parameter;
use indexmap::IndexMap;
use stdweb::traits::IEvent;
use yew::prelude::*;
use yew::virtual_dom::VNode;

#[derive(Clone, Default, PartialEq)]
pub struct Properties {
    pub name: String,
    pub is_expanded: bool,
    pub parameters: IndexMap<String, bool>,
    pub onexpandchange: Option<Callback<bool>>,
    pub onchange: Option<Callback<(String, bool)>>,
}

pub struct Feature {
    name: String,
    is_expanded: bool,
    parameters: IndexMap<String, bool>,
    onexpandchange: Option<Callback<bool>>,
    onchange: Option<Callback<(String, bool)>>,
}

pub enum Message {
    Toggle,
    SetParameter { name: String, is_selected: bool },
}

impl Component for Feature {
    type Message = Message;
    type Properties = Properties;

    fn create(properties: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self {
            name: properties.name,
            is_expanded: properties.is_expanded,
            parameters: properties.parameters,
            onexpandchange: properties.onexpandchange,
            onchange: properties.onchange,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        match message {
            Message::Toggle => {
                if let Some(ref mut callback) = self.onexpandchange {
                    callback.emit(!self.is_expanded);
                }
            }
            Message::SetParameter { name, is_selected } => {
                if let Some(ref mut callback) = self.onchange {
                    callback.emit((name, is_selected));
                }
            }
        }
        false
    }

    fn change(&mut self, properties: Self::Properties) -> ShouldRender {
        let changed = self.name != properties.name
            || self.is_expanded != properties.is_expanded
            || self.parameters != properties.parameters;
        self.name = properties.name;
        self.is_expanded = properties.is_expanded;
        self.parameters = properties.parameters;
        self.onchange = properties.onchange;
        self.onexpandchange = properties.onexpandchange;
        changed
    }
}

impl Renderable<Feature> for Feature {
    fn view(&self) -> Html<Self> {
        let mut tag = html! {
            <details>
                <summary onclick=|e| { e.prevent_default(); Message::Toggle },>{ &self.name }</summary>
                // capitalize `type` because otherwise yew eats it
                <div Type="checkbox", class="sdpi-item",>
                    <div class="sdpi-item-value min100",>
                        { for self.parameters.iter().map(|(name, is_selected)| {
                            let cb_name = name.clone();
                            html! {
                                <Parameter: name=name, is_selected=is_selected, onchange=move |is_selected| { Message::SetParameter { name: cb_name.clone(), is_selected } }, />
                            }
                        }) }
                    </div>
                </div>
            </details>
        };
        if self.is_expanded {
            if let VNode::VTag(ref mut vtag) = tag {
                vtag.add_attribute("open", &"");
            }
        }
        tag
    }
}
