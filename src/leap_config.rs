use std::{collections::BTreeMap, str::FromStr};
use strum::EnumString;

pub struct LeapConfig {
    pub target: LeapTargetKind,
    pub pane_unfocus_behaviour: PaneUnfocusBehaviour,
    pub escape_behavior: EscapeBehavior,
}

impl Default for LeapConfig {
    fn default() -> Self {
        Self {
            target: LeapTargetKind::Tab,
            // TODO: set default to Close
            pane_unfocus_behaviour: PaneUnfocusBehaviour::None,
            escape_behavior: EscapeBehavior::Close,
        }
    }
}

#[derive(EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum LeapTargetKind {
    Tab,
    TabExceptActive,
    PaneInActiveTab,
    // TODO: PaneAcrossTabs?
    Session,
}

#[derive(EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum PaneUnfocusBehaviour {
    None,
    Close,
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
            target: Self::parse_str_enum(&configuration, "leap_target", default.target),
            pane_unfocus_behaviour: Self::parse_str_enum(
                &configuration,
                "leap_on_pane_unfocus",
                default.pane_unfocus_behaviour,
            ),
            escape_behavior: Self::parse_str_enum(
                &configuration,
                "leap_on_escape",
                default.escape_behavior,
            ),
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
