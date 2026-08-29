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
    pub is_pane_focused: bool,
    pub error: Option<String>,
}
