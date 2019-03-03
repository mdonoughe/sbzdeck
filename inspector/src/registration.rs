use serde::de;
use serde_derive::Deserialize;
use std::fmt;

/// Information about a connected device.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/registration-procedure/#info-parameter)
#[derive(Debug, Deserialize)]
pub struct RegistrationInfoDevice {
    /// The ID of the specific device.
    pub id: String,
    /// The size of the device.
    pub size: DeviceSize,
    /// The type of the device.
    #[serde(rename = "type")]
    pub _type: Option<DeviceType>,
}

/// The language the Stream Deck software is running in.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/registration-procedure/#Info-parameter)
#[derive(Debug)]
pub enum Language {
    English,
    French,
    German,
    Spanish,
    Japanese,
    /// Unlike the other lanuages which are not specifically localized to a country, Chinese is specifically zh-CN.
    ChineseChina,
    /// A language that was not documented in the 4.0.0 SDK.
    Unknown(String),
}

impl<'de> de::Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Language;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Language, E>
            where
                E: de::Error,
            {
                Ok(match value {
                    "en" => Language::English,
                    "fr" => Language::French,
                    "de" => Language::German,
                    "es" => Language::Spanish,
                    "ja" => Language::Japanese,
                    "zh_cn" => Language::ChineseChina,
                    value => Language::Unknown(value.to_string()),
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// The platform on which the Stream Deck software is running.
#[derive(Debug)]
pub enum Platform {
    /// Mac OS X
    Mac,
    /// Windows
    Windows,
    /// A platform not documented in the 4.0.0 SDK.
    Unknown(String),
}

impl<'de: 'a, 'a> de::Deserialize<'de> for Platform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Platform;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Platform, E>
            where
                E: de::Error,
            {
                Ok(match value {
                    "mac" => Platform::Mac,
                    "windows" => Platform::Windows,
                    value => Platform::Unknown(value.to_string()),
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// Information about the Stream Deck software.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/registration-procedure/#info-parameter)
#[derive(Debug, Deserialize)]
pub struct RegistrationInfoApplication {
    pub language: Language,
    pub platform: Platform,
    pub version: String,
}

/// Information about the environment the plugin is being loaded into.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/registration-procedure/#info-parameter)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationInfo {
    pub application: RegistrationInfoApplication,
    pub device_pixel_ratio: u8,
    pub devices: Vec<RegistrationInfoDevice>,
}

/// Information about the action for this property inspector.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/registration-procedure/#inactioninfo-parameter)
#[derive(Debug, Deserialize)]
pub struct ActionInfo<S> {
    pub action: String,
    pub context: String,
    pub device: String,
    pub payload: ActionInfoPayload<S>,
}

/// Additional information about the action.
#[derive(Debug, Deserialize)]
pub struct ActionInfoPayload<S> {
    /// The stored settings for the action instance.
    pub settings: S,
    /// The location of the key that was pressed, or None if this action instance is part of a multi action.
    pub coordinates: Option<Coordinates>,
}

/// The location of a key on a device.
///
/// Locations are specified using zero-indexed values starting from the top left corner of the device.
#[derive(Debug, Deserialize)]
pub struct Coordinates {
    /// The x coordinate of the key.
    pub column: u8,
    /// The y-coordinate of the key.
    pub row: u8,
}

/// The size of a device in keys.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSize {
    /// The number of key columns on the device.
    pub columns: u8,
    /// The number of key rows on the device.
    pub rows: u8,
}

/// The type of connected hardware device.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/overview/#stream-deck-hardware)
#[derive(Debug)]
pub enum DeviceType {
    /// The [Stream Deck](https://www.elgato.com/en/gaming/stream-deck).
    StreamDeck,
    /// The [Stream Deck Mini](https://www.elgato.com/en/gaming/stream-deck-mini).
    StreamDeckMini,
    /// A device not documented in the 4.0.0 SDK.
    Unknown(u64),
}

impl<'de> de::Deserialize<'de> for DeviceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DeviceType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an integer")
            }

            fn visit_u64<E>(self, value: u64) -> Result<DeviceType, E>
            where
                E: de::Error,
            {
                Ok(match value {
                    0 => DeviceType::StreamDeck,
                    1 => DeviceType::StreamDeckMini,
                    value => DeviceType::Unknown(value),
                })
            }
        }

        deserializer.deserialize_u64(Visitor)
    }
}

pub struct InspectorRegistrationParams<S> {
    pub url: String,
    pub property_inspector_uuid: String,
    pub register_event: String,
    pub info: RegistrationInfo,
    pub action_info: ActionInfo<S>,
}
