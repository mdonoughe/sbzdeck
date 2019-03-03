use super::message::{Message, MessageOut};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_derive::Serialize;
use std::marker::PhantomData;
use yew::callback::Callback;
use yew::format::{Binary, Text};
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

pub struct StreamDeckSocketService<G, S, MI, MO> {
    inner: WebSocketService,
    _phantom: PhantomData<(G, S, MI, MO)>,
}

pub struct StreamDeckSocketTask<G, S, MO> {
    inner: WebSocketTask,
    _phantom: PhantomData<(G, S, MO)>,
}

impl<G, S, MO> StreamDeckSocketTask<G, S, MO>
where
    G: Serialize,
    S: Serialize,
    MO: Serialize,
{
    pub fn send(&mut self, data: &MessageOut<G, S, MO>) {
        let message = serde_json::to_string(data).unwrap();
        self.inner.send(Ok(message));
    }

    pub fn register(&mut self, event: &str, uuid: &str) {
        let registration = serde_json::to_string(&Registration { event, uuid }).unwrap();
        self.inner.send(Ok(registration));
    }
}

struct WsMessage<G, S, MI> {
    pub message: Message<G, S, MI>,
}

impl<G, S, MI> From<Binary> for WsMessage<G, S, MI> {
    fn from(_input: Binary) -> Self {
        panic!("unexpected binary data")
    }
}

impl<G, S, MI> From<Text> for WsMessage<G, S, MI>
where
    G: DeserializeOwned,
    S: DeserializeOwned,
    MI: DeserializeOwned,
{
    fn from(input: Text) -> Self {
        WsMessage {
            message: serde_json::from_str(&input.unwrap()).unwrap(),
        }
    }
}

#[derive(Serialize)]
struct Registration<'a> {
    event: &'a str,
    uuid: &'a str,
}

impl<G, S, MI, MO> StreamDeckSocketService<G, S, MI, MO>
where
    G: 'static + DeserializeOwned,
    S: 'static + DeserializeOwned,
    MI: 'static + DeserializeOwned,
{
    pub fn new() -> Self {
        Self {
            inner: WebSocketService::new(),
            _phantom: PhantomData,
        }
    }

    pub fn connect(
        &mut self,
        address: &str,
        callback: Callback<Message<G, S, MI>>,
        notification: Callback<WebSocketStatus>,
    ) -> StreamDeckSocketTask<G, S, MO> {
        let task = self.inner.connect(
            address,
            Callback::from(move |message: WsMessage<G, S, MI>| callback.emit(message.message)),
            Callback::from(move |status| notification.emit(status)),
        );
        StreamDeckSocketTask {
            inner: task,
            _phantom: PhantomData,
        }
    }
}
