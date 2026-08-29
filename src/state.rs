use crate::{config::*, matching::*, utils::*};
use zellij_tile::prelude::*;

#[derive(Clone)]
pub enum LeapLocation {
    Tab {
        tab_index: TabIndex,
    },
    Pane {
        pane_id: PaneId,
        is_floating: bool,
        is_suppressed: bool,
    },
    Session {
        session_name: SessionName,
        is_alive: bool,
    },
}

pub enum SequenceDirection {
    Prev,
    Next,
}

impl SequenceDirection {
    pub fn advance_index(&self, index: usize, len: usize) -> usize {
        match self {
            Self::Prev if index == 0 => len - 1,
            Self::Prev => index - 1,
            Self::Next if index + 1 == len => 0,
            Self::Next => index + 1,
        }
    }
}

pub struct LeapTarget {
    pub name: MatchedString,
    pub being_matched: Resettable<bool>,
    pub current: bool,
    pub location: LeapLocation,
}

#[derive(Default)]
pub struct LeapState {
    pub config: LeapConfig,
    pub targets: Vec<LeapTarget>,
    pub manual_selection: Option<usize>,
    pub last_panes: Option<PaneManifest>,
    pub last_tabs: Option<Vec<TabInfo>>,
    pub last_sessions: Option<SessionListSnapshot>,
    pub is_pane_focused: bool,
    pub error: Option<String>,
}
