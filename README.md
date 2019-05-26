# sbzdeck

> [Stream Deck](https://www.elgato.com/en/gaming/stream-deck) plugin for controlling [Sound Blaster](https://www.soundblaster.com/products/soundcards) cards

![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg) [![Build status](https://travis-ci.org/mdonoughe/sbzdeck.svg)](https://travis-ci.org/mdonoughe/sbzdeck/)

Right now this plugin only contains a single action, Select Output, which toggles between the headphone and speaker out of the default sound card if the default sound card supports Creative's Sound Blaster control interface.

This plugin is probably only useful for a few people in the world.

## Requirements

- Windows 10
- Stream Deck software version 4.1.0
- The Sound Blaster device must be the Windows default audio output (selecting a target device is not yet supported)

## Usage

1. Download and open the streamDeckPlugin file from the [Releases](https://github.com/mdonoughe/sbzdeck/releases) section.
2. In the Stream Deck software, drag the Select Output action from the new sbzdeck category onto the Stream Deck screen.

Pressing the button on the Stream Deck will cause the output to switch.

The current settings will be remembered when switching, and will be restored when switching back. By default, only the volume and SBX Pro Studio switch are applied. See the configuration section.

It is also possible to create a Stream Deck "multi action" which uses the plugin to select specifically headphones or speakers rather than toggling, in case you want to do something like always use headphones while recording.

## Configuration

When the plugin is selected in the Stream Deck software, the property inspector in the bottom panel of the window will display a list of features and their associated parameters. Only the parameters that are checked in this list will be restored when switching inputs.

## Icons

The shapes in the icons come from the [Material Design Icon Library](https://material.io/tools/icons/). The style of the key icons is supposed to look like Creative's icons.
