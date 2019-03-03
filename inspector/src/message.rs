use serde::de;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

/// A message received from the Stream Deck software.
///
/// - `G` represents the global settings that are persisted within the Stream Deck software.
/// - `S` represents the settings that are persisted within the Stream Deck software.
/// - `M` represents the messages that are received from the property inspector.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-received/)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum Message<G, S, M> {
    /// The property inspector has sent data.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-received/#sendtoplugin)
    #[serde(rename_all = "camelCase")]
    SendToPropertyInspector {
        /// The uuid of the action.
        action: String,
        /// The instance of the action (key or part of a multiaction).
        context: String,
        /// Information sent from the property inspector.
        payload: M,
    },
    /// The application has sent settings for an action.
    ///
    /// This message is sent in response to GetSettings, but also after the
    /// property inspector changes the settings.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-received/#didreceivesettings)
    #[serde(rename_all = "camelCase")]
    DidReceiveSettings {
        /// The uuid of the action.
        action: String,
        /// The instance of the action (key or part of a multiaction).
        context: String,
        /// The device where the action exists.
        device: String,
        /// The current settings for the action.
        payload: KeyPayload<S>,
    },
    /// The application has sent settings for an action.
    ///
    /// This message is sent in response to GetGlobalSettings, but also after
    /// the property inspector changes the settings.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-received/#didreceiveglobalsettings)
    #[serde(rename_all = "camelCase")]
    DidReceiveGlobalSettings {
        /// The current settings for the action.
        payload: GlobalSettingsPayload<G>,
    },
}

/// A message to be sent to the Stream Deck software.
///
/// - `G` represents the global settings that are persisted within the Stream Deck software.
/// - `S` represents the action settings that are persisted within the Stream Deck software.
/// - `M` represents the messages that are sent to the property inspector.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/)
#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum MessageOut<G, S, M> {
    /// Retrieve settings for an instance of an action via DidReceiveSettings.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/#getsettings)
    #[serde(rename_all = "camelCase")]
    GetSettings {
        /// The instance of the action (key or part of a multiaction).
        context: String,
    },
    /// Store settings for an instance of an action.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/#setsettings)
    #[serde(rename_all = "camelCase")]
    SetSettings {
        /// The instance of the action (key or part of a multiaction).
        context: String,
        /// The settings to save.
        payload: S,
    },
    /// Send data to the plugin.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/#sendtoplugin)
    #[serde(rename_all = "camelCase")]
    SendToPlugin {
        /// The uuid of the action.
        action: String,
        /// The instance of the action (key or part of a multiaction).
        context: String,
        /// The message to send.
        payload: M,
    },
    /// Open a URL in the default browser.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/#openurl)
    #[serde(rename_all = "camelCase")]
    OpenUrl {
        /// The url to open.
        payload: UrlPayload,
    },
    /// Retrieve plugin settings for via DidReceiveGlobalSettings.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/#getglobalsettings)
    #[serde(rename_all = "camelCase")]
    GetGlobalSettings {
        /// The instance of the action (key or part of a multiaction).
        context: String,
    },
    /// Store plugin settings.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/#setglobalsettings)
    #[serde(rename_all = "camelCase")]
    SetGlobalSettings {
        /// The instance of the action (key or part of a multiaction).
        context: String,
        /// The settings to save.
        payload: G,
    },
    /// Write to the log.
    ///
    /// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/#logmessage)
    #[serde(rename_all = "camelCase")]
    LogMessage {
        /// The message to log.
        payload: LogMessagePayload,
    },
}

/// The URL to launch as part of a [OpenUrl](enum.MessageOut.html#variant.OpenUrl) message.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-sent/#openurl)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlPayload {
    /// The URL to launch.
    pub url: String,
}

/// Additional information about the key pressed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPayload<S> {
    /// The stored settings for the action instance.
    pub settings: S,
    /// The location of the key that was pressed, or None if this action instance is part of a multi action.
    pub coordinates: Option<Coordinates>,
    /// The current state of the action instance.
    pub state: u8,
    /// The desired state of the action instance (if this instance is part of a multi action).
    pub user_desired_state: Option<u8>,
    //TODO: is_in_multi_action ignored. replace coordinates with enum Location { Coordinates, MultiAction }.
}

/// The new global settings.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettingsPayload<G> {
    /// The stored settings for the plugin.
    pub settings: G,
}

/// A log message.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogMessagePayload {
    /// The log message text.
    pub message: String,
}

/// Information about a hardware device.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/registration-procedure/#Info-parameter)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// The size of the device.
    pub size: DeviceSize,
    /// The type of the device, or None if the Stream Deck software is running with no device attached.
    #[serde(rename = "type")]
    pub _type: Option<DeviceType>,
}

/// Information about a monitored application that has launched or terminated.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationPayload {
    /// The name of the application.
    pub application: String,
}

/// The location of a key on a device.
///
/// Locations are specified using zero-indexed values starting from the top left corner of the device.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coordinates {
    /// The x coordinate of the key.
    pub column: u8,
    /// The y-coordinate of the key.
    pub row: u8,
}

/// The vertical alignment of a title.
///
/// Titles are always centered horizontally.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Alignment {
    /// The title should appear at the top of the key.
    Top,
    /// The title should appear in the middle of the key.
    Middle,
    /// The title should appear at the bottom of the key.
    Bottom,
}

/// Style information for a title.
///
/// [Official Documentation](https://developer.elgato.com/documentation/stream-deck/sdk/events-received/#titleparametersdidchange)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleParameters {
    /// The name of the font family.
    pub font_family: String,
    /// The font size.
    pub font_size: u8,
    /// Whether the font is bold and/or italic.
    pub font_style: String,
    /// Whether the font is underlined.
    pub font_underline: bool,
    /// Whether the title is displayed.
    pub show_title: bool,
    /// The vertical alignment of the title.
    pub title_alignment: Alignment,
    /// The color of the title.
    pub title_color: String,
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
