use std::{collections::BTreeMap, str::FromStr};
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};

pub struct LeapConfig {
    pub target: LeapTargetKind,
    pub no_match_behavior: NoMatchBehavior,
    pub pane_unfocus_behavior: PaneUnfocusBehavior,
    pub escape_behavior: EscapeBehavior,
}

impl Default for LeapConfig {
    fn default() -> Self {
        Self {
            target: LeapTargetKind::Tab,
            no_match_behavior: NoMatchBehavior::Reset,
            pane_unfocus_behavior: PaneUnfocusBehavior::None,
            escape_behavior: EscapeBehavior::Close,
        }
    }
}

#[derive(EnumString, EnumIter, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum LeapTargetKind {
    Tab,
    TabExceptActive,
    PaneInActiveTab,
    // TODO: PaneAcrossTabs?
    Session,
    SessionExceptCurrent,
}

#[derive(EnumString, EnumIter, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum NoMatchBehavior {
    Reset,
    Close,
    HideFloatingPanes,
}

#[derive(EnumString, EnumIter, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum PaneUnfocusBehavior {
    None,
    Close,
}

#[derive(EnumString, EnumIter, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum EscapeBehavior {
    Close,
    HideFloatingPanes,
}

impl LeapConfig {
    pub fn parse(configuration: BTreeMap<String, String>) -> Result<Self, String> {
        let default = LeapConfig::default();

        Ok(Self {
            target: Self::parse_str_enum(&configuration, "leap_target", default.target)?,
            no_match_behavior: Self::parse_str_enum(
                &configuration,
                "leap_on_no_match",
                default.no_match_behavior,
            )?,
            pane_unfocus_behavior: Self::parse_str_enum(
                &configuration,
                "leap_on_pane_unfocus",
                default.pane_unfocus_behavior,
            )?,
            escape_behavior: Self::parse_str_enum(
                &configuration,
                "leap_on_escape",
                default.escape_behavior,
            )?,
        })
    }

    fn parse_str_enum<E: FromStr + IntoEnumIterator + Into<&'static str>>(
        configuration: &BTreeMap<String, String>,
        key: &str,
        default: E,
    ) -> Result<E, String> {
        let Some(config_value) = configuration.get(key) else {
            return Ok(default);
        };

        E::from_str(config_value).map_err(|_| {
            format!(
                "{key}: '{}' expected, got '{}'",
                E::iter()
                    .map(|possible_value| possible_value.into())
                    .collect::<Vec<_>>()
                    .join("' | '"),
                config_value,
            )
        })
    }
}
