# streamdeck-rs

> [Stream Deck](https://www.elgato.com/en/gaming/stream-deck) plugin for controlling [Sound Blaster](https://www.soundblaster.com/products/soundcards) cards

![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg) [![Build status](https://travis-ci.org/mdonoughe/sbzdeck.svg)](https://travis-ci.org/mdonoughe/sbzdeck/)

Right now this plugin only contains a single action, Select Output, which toggles between the headphone and speaker out of the default sound card if the default sound card supports Creative's Sound Blaster control interface.

This plugin is probably only useful for a few people in the world.

## Requirements

- Windows 10
- Stream Deck software version 4.0.0
- The Sound Blaster device must be the Windows default audio output (selecting a target device is not yet supported)

## Usage

1. Download and open the streamDeckPlugin file from the [Releases](https://github.com/mdonoughe/sbzdeck/releases) section.
2. In the Stream Deck software, drag the Select Output action from the new sbzdeck category onto the Stream Deck screen.

Pressing the button on the Stream Deck will cause the output to switch.

The current settings will be remembered when switching, and will be restored when switching back. By default, only the volume and SBX Pro Studio switch are applied. See the configuration section.

It is also possible to create a Stream Deck "multi action" which uses the plugin to select specifically headphones or speakers rather than toggling, in case you want to do something like always use headphones while recording.

## Configuration

Currently, the only way to configure the plugin is by editing the `sbzdeck.json` file found in the `%APPDATA%\Elgato\StreamDeck\Plugins\io.github.mdonoughe.sbzdeck.sdPlugin` directory after installing the plugin, and then restarting the Stream Deck software.

### `profiles`

The profiles section contains the settings that are applied when switching between headphones and speakers.

These are only the defaults values, and they are overwritten right before switching to the other output. For example, if the JSON file says the speaker volume should be 0.5, but you've adjusted the volume to 0.6, switching to headphones and then back to speakers will set the volume back to 0.6, not 0.5.

The parameters subsection contains the Sound Blaster settings. You can use the [sbz-switch](https://github.com/mdonoughe/sbz-switch/) dump command to find settings you want. Note that setting these values here sets the default values, but the settings will not be applied unless the parameter is selected in the `selected_parameters` section, because the idea is that this section is supposed to be constantly updated by the plugin to contain all the settings (not implemented).

### `selected_parameters`

These are the parameters that are applied when switching outputs. You can use the [sbz-switch](https://github.com/mdonoughe/sbz-switch/) dump command to find settings you want.

The sound volume is always applied.
