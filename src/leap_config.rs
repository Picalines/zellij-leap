use std::{collections::BTreeMap, str::FromStr};
use strum::EnumString;

pub struct LeapConfig {
    pub include_current_target: bool,
    pub close_on_pane_unfocus: bool,
    pub escape_behavior: EscapeBehavior,
}

impl Default for LeapConfig {
    fn default() -> Self {
        Self {
            include_current_target: true,
            close_on_pane_unfocus: true,
            escape_behavior: EscapeBehavior::Close,
        }
    }
}

#[derive(EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum EscapeBehavior {
    Close,
    HideFloatingPanes,
}

impl LeapConfig {
    // TODO: error reporting could be nice
    pub fn parse(configuration: BTreeMap<String, String>) -> Self {
        let default = LeapConfig::default();

        Self {
            include_current_target: Self::parse_bool_pair(
                &configuration,
                "leap_include_current_target",
                default.include_current_target,
            ),
            close_on_pane_unfocus: Self::parse_bool_pair(
                &configuration,
                "leap_close_on_pane_unfocus",
                default.close_on_pane_unfocus,
            ),
            escape_behavior: Self::parse_str_enum(
                &configuration,
                "leap_on_escape",
                default.escape_behavior,
            ),
        }
    }

    fn parse_bool_pair(configuration: &BTreeMap<String, String>, key: &str, default: bool) -> bool {
        let Some(config_value) = configuration.get(key) else {
            return default;
        };

        if config_value == "true" {
            true
        } else if config_value == "false" {
            false
        } else {
            default
        }
    }

    fn parse_str_enum<E: FromStr>(
        configuration: &BTreeMap<String, String>,
        key: &str,
        default: E,
    ) -> E {
        let Some(config_value) = configuration.get(key) else {
            return default;
        };

        E::from_str(config_value).unwrap_or(default)
    }
}
