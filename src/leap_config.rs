use std::collections::BTreeMap;

pub struct LeapConfig {
    pub include_current_target: bool,
    pub close_on_pane_unfocus: bool,
}

impl Default for LeapConfig {
    fn default() -> Self {
        Self {
            include_current_target: true,
            close_on_pane_unfocus: true,
        }
    }
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
}
