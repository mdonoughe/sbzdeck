use futures::sync::mpsc;
use indexmap::{IndexMap, IndexSet};
use sbz_switch::soundcore::SoundCoreParamValue;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ops::{Index, IndexMut};
use std::sync::{Arc, Mutex};
use streamdeck_rs::MessageOut;

#[derive(Debug, Deserialize, Serialize)]
pub struct Empty {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Output {
    Headphones,
    Speakers,
}

impl Into<u8> for Output {
    fn into(self) -> u8 {
        match self {
            Output::Headphones => 0,
            Output::Speakers => 1,
        }
    }
}

impl Output {
    pub fn try_from(value: &SoundCoreParamValue) -> Option<Self> {
        match value {
            SoundCoreParamValue::U32(0) => Some(Output::Headphones),
            SoundCoreParamValue::U32(1) => Some(Output::Speakers),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct Profile {
    pub volume: Option<f32>,
    pub parameters: IndexMap<String, IndexMap<String, SoundCoreParamValue>>,
}

#[derive(Debug, Default)]
pub struct Profiles {
    pub headphones: Profile,
    pub speakers: Profile,
}

impl Index<Output> for Profiles {
    type Output = Profile;

    fn index(&self, index: Output) -> &Profile {
        match index {
            Output::Headphones => &self.headphones,
            Output::Speakers => &self.speakers,
        }
    }
}

impl IndexMut<Output> for Profiles {
    fn index_mut(&mut self, index: Output) -> &mut Profile {
        match index {
            Output::Headphones => &mut self.headphones,
            Output::Speakers => &mut self.speakers,
        }
    }
}

pub struct RawState {
    pub output: Option<Output>,
    pub contexts: BTreeSet<String>,
    pub out: mpsc::Sender<MessageOut<Empty, Empty>>,
    pub settings: CardSettings,
}

pub type State = Arc<Mutex<RawState>>;

#[derive(Debug, Default)]
pub struct CardSettings {
    pub selected_parameters: IndexMap<String, IndexSet<String>>,
    pub profiles: Profiles,
}
