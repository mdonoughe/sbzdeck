use crate::types::*;
use slog::Logger;

pub fn get_current_profile(
    logger: &Logger,
) -> Result<Option<(Output, Profile)>, Box<dyn std::error::Error>> {
    match sbz_switch::dump(logger, None) {
        Ok(device_state) => Ok(match device_state.creative {
            Some(creative) => {
                let output = creative
                    .get("Device Control")
                    .and_then(|control| control.get("SelectOutput").and_then(Output::try_from));
                match output {
                    Some(output) => Some((
                        output,
                        Profile {
                            volume: device_state
                                .endpoint
                                .as_ref()
                                .and_then(|endpoint| endpoint.volume),
                            parameters: creative,
                        },
                    )),
                    None => None,
                }
            }
            None => None,
        }),
        Err(error) => Err(error),
    }
}
