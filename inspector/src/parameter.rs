use stdweb::traits::*;
use yew::prelude::*;

#[derive(Clone, Default, PartialEq)]
pub struct Properties {
    pub name: String,
    pub is_selected: bool,
    pub onchange: Option<Callback<bool>>,
}

pub struct Parameter {
    name: String,
    is_selected: bool,
    onchange: Option<Callback<bool>>,
}

pub enum Message {
    Toggle,
}

impl Component for Parameter {
    type Message = Message;
    type Properties = Properties;

    fn create(properties: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self {
            name: properties.name,
            is_selected: properties.is_selected,
            onchange: properties.onchange,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        match message {
            Message::Toggle => {
                if let Some(ref mut callback) = self.onchange {
                    callback.emit(!self.is_selected)
                }
            }
        }
        false
    }

    fn change(&mut self, properties: Self::Properties) -> ShouldRender {
        let changed = &self.name != &properties.name || self.is_selected != properties.is_selected;
        self.name = properties.name;
        self.is_selected = properties.is_selected;
        self.onchange = properties.onchange;
        changed
    }
}

impl Renderable<Parameter> for Parameter {
    fn view(&self) -> Html<Self> {
        html! {
            <div class="sdpi-item-child", onclick=|e| { e.prevent_default(); Message::Toggle },>
                <input id=&self.name, type="checkbox", checked=self.is_selected,/>
                <label for=&self.name, class="sdpi-item-label",><span></span>{ &self.name }</label>
            </div>
        }
    }
}
