mod config;
mod matching;
mod state;
mod ui;
mod utils;

use std::collections::BTreeMap;
use std::time::Duration;
use zellij_tile::prelude::*;

use crate::config::*;
use crate::matching::*;
use crate::state::*;
use crate::utils::*;

register_plugin!(LeapState);

impl ZellijPlugin for LeapState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        match LeapConfig::parse(configuration) {
            Ok(config) => self.config = config,
            Err(error) => self.error = Some(error),
        }

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        subscribe(&[
            EventType::Key,
            EventType::PaneUpdate,
            EventType::PermissionRequestResult,
            EventType::TabUpdate,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::PaneUpdate(panes) => self.handle_pane_update(panes),
            Event::PermissionRequestResult(permissions) => {
                self.handle_permissions_update(permissions)
            }
            Event::TabUpdate(tabs) => self.handle_tab_update(tabs),
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        ui::render(self, rows, cols)
    }
}

impl LeapState {
    fn handle_permissions_update(&mut self, permission_status: PermissionStatus) -> bool {
        if !matches!(permission_status, PermissionStatus::Granted) {
            self.error = Some("permissions not granted".to_string());
            return true;
        }

        rename_plugin_pane(get_plugin_ids().plugin_id, "leap");

        match self.config.target {
            LeapTargetKind::Session | LeapTargetKind::SessionExceptCurrent => {
                match get_session_list() {
                    Err(error) => {
                        self.error = Some(format!("failed to fetch session list: {}", error));
                        true
                    }
                    Ok(sessions) => {
                        self.last_sessions = Some(sessions);
                        self.refresh()
                    }
                }
            }
            _ => self.refresh(),
        }
    }

    fn handle_tab_update(&mut self, tabs: Vec<TabInfo>) -> bool {
        self.last_tabs = Some(tabs);
        self.refresh()
    }

    fn handle_pane_update(&mut self, panes: PaneManifest) -> bool {
        self.last_panes = Some(panes);
        self.refresh()
    }

    fn refresh(&mut self) -> bool {
        self.refresh_pane_focus();

        if self.is_pane_focused && !self.targets.is_empty() {
            return false;
        }

        self.manual_selection = None;
        self.targets = self.collect_targets();
        true
    }

    fn collect_targets(&self) -> Vec<LeapTarget> {
        match self.config.target {
            LeapTargetKind::Tab | LeapTargetKind::TabExceptActive => {
                let match_active = matches!(self.config.target, LeapTargetKind::Tab);
                let tabs = self.last_tabs.as_deref().unwrap_or_default();
                self.tab_targets(tabs.iter(), match_active)
            }
            LeapTargetKind::PaneInActiveTab => {
                let active_tab_position = self
                    .last_tabs
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .find(|tab| tab.active)
                    .map(|tab| tab.position);

                let panes = active_tab_position
                    .and_then(|tab_position| {
                        self.last_panes
                            .as_ref()?
                            .panes
                            .get(&tab_position)
                            .map(Vec::as_slice)
                    })
                    .unwrap_or_default();

                self.pane_targets(panes.iter())
            }
            LeapTargetKind::Session | LeapTargetKind::SessionExceptCurrent => {
                let match_current = matches!(self.config.target, LeapTargetKind::Session);
                let (live_sessions, resurrectable_sessions) = self
                    .last_sessions
                    .as_ref()
                    .map(|sessions| {
                        (
                            sessions.live_sessions.as_slice(),
                            sessions.resurrectable_sessions.as_slice(),
                        )
                    })
                    .unwrap_or_default();

                self.session_targets(
                    live_sessions.iter(),
                    resurrectable_sessions.iter(),
                    match_current,
                )
            }
        }
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        enum Mods {
            None,
            Ctrl,
            Shift,
        }

        let mods = match () {
            _ if key.has_modifiers(&[KeyModifier::Ctrl]) => Mods::Ctrl,
            _ if key.has_modifiers(&[KeyModifier::Shift]) => Mods::Shift,
            _ => Mods::None,
        };

        match (key.bare_key, mods) {
            (BareKey::Esc, _) => self.handle_escape(),
            (BareKey::Enter, _) => self.handle_enter(),
            (BareKey::Tab, Mods::None) => {
                self.move_selection_to_next_match(SequenceDirection::Next);
                true
            }
            (BareKey::Tab, Mods::Shift) => {
                self.move_selection_to_next_match(SequenceDirection::Prev);
                true
            }
            (BareKey::Up, _) | (BareKey::Char('k' | 'p'), Mods::Ctrl) => {
                self.move_manual_selection(SequenceDirection::Prev);
                true
            }
            (BareKey::Down, _) | (BareKey::Char('j' | 'n'), Mods::Ctrl) => {
                self.move_manual_selection(SequenceDirection::Next);
                true
            }
            (BareKey::Char('u'), Mods::Ctrl) => self.reset_matching(),
            (BareKey::Char(ch), Mods::None) => {
                self.handle_char_key(ch);
                true
            }
            _ => false,
        }
    }

    fn handle_char_key(&mut self, ch: char) {
        if self.error.is_some() {
            return;
        }

        let mut number_of_matches = 0usize;
        let mut first_matched_index: Option<usize> = None;
        let mut last_matched_location: Option<LeapLocation> = None;

        for (index, target) in self.targets.iter_mut().enumerate() {
            if !target.being_matched.current {
                continue;
            }

            if target.name.match_char(ch) {
                number_of_matches += 1;
                first_matched_index = first_matched_index.or(Some(index));
                last_matched_location = Some(target.location.clone());
            } else {
                target.being_matched.current = false;
            }
        }

        self.manual_selection = first_matched_index.or(self.manual_selection);

        match (number_of_matches, last_matched_location) {
            (0, _) => self.handle_no_matches(),
            (1, Some(leap_location)) => self.switch_to_location(&leap_location),
            _ => (),
        };
    }

    fn handle_enter(&mut self) -> bool {
        match self.assumed_selection() {
            Some(target_index) => {
                let leap_location = self.targets[target_index].location.clone();
                self.switch_to_location(&leap_location);
                true
            }
            None => false,
        }
    }

    fn move_manual_selection(&mut self, dir: SequenceDirection) {
        let Some(target_index) = self.assumed_selection() else {
            return;
        };

        self.manual_selection = Some(dir.advance_index(target_index, self.targets.len()));
    }

    fn move_selection_to_next_match(&mut self, dir: SequenceDirection) {
        let Some(mut selection_index) = self.assumed_selection() else {
            return;
        };

        let matched_index = (0..self.targets.len()).find_map(|_| {
            selection_index = dir.advance_index(selection_index, self.targets.len());
            self.targets[selection_index]
                .being_matched
                .current
                .then_some(selection_index)
        });
        self.manual_selection = matched_index.or(self.manual_selection);
    }

    fn switch_to_location(&mut self, leap_location: &LeapLocation) {
        self.handle_matched();

        match leap_location {
            LeapLocation::Tab { tab_index, .. } => switch_tab_to(tab_index.0 as u32 + 1),
            LeapLocation::Pane { pane_id, .. } => focus_pane_with_id(*pane_id, false, false),
            LeapLocation::Session { session_name, .. } => switch_session(Some(&session_name.0)),
        }
    }

    fn handle_matched(&mut self) {
        // TODO: matched behavior option?
        self.targets.clear();
        hide_floating_panes_in_active_tab();
        close_self();
    }

    fn handle_no_matches(&mut self) {
        match self.config.no_match_behavior {
            NoMatchBehavior::Reset => {
                self.reset_matching();
            }
            NoMatchBehavior::Close => close_self(),
            NoMatchBehavior::HideFloatingPanes => hide_floating_panes_in_active_tab(),
        }
    }

    fn handle_escape(&mut self) -> bool {
        match self.config.escape_behavior {
            EscapeBehavior::Close => {
                close_self();
                false
            }
            EscapeBehavior::ResetOrClose => {
                self.reset_matching() || {
                    close_self();
                    false
                }
            }
            EscapeBehavior::HideFloatingPanes => {
                hide_floating_panes_in_active_tab();
                false
            }
            EscapeBehavior::ResetOrHideFloatingPanes => {
                self.reset_matching() || {
                    hide_floating_panes_in_active_tab();
                    false
                }
            }
        }
    }

    fn tab_targets<'a>(
        &self,
        tabs: impl Iterator<Item = &'a TabInfo>,
        match_active: bool,
    ) -> Vec<LeapTarget> {
        tabs.map(|tab| LeapTarget {
            name: MatchedString::new(tab.name.clone()),
            being_matched: Resettable::new(!tab.active || match_active),
            current: tab.active,
            location: LeapLocation::Tab {
                tab_index: TabIndex(tab.position),
            },
        })
        .collect()
    }

    fn pane_targets<'a>(&self, panes: impl Iterator<Item = &'a PaneInfo>) -> Vec<LeapTarget> {
        let self_plugin_id = get_plugin_ids().plugin_id;
        let self_plugin_url = self.last_panes.as_ref().and_then(|pane_manifest| {
            pane_manifest
                .panes
                .values()
                .flatten()
                .find(|pane| pane.is_plugin && pane.id == self_plugin_id)
                .and_then(|pane| pane.plugin_url.as_deref())
        });

        panes
            .filter_map(|pane| {
                let is_leap_plugin = pane.is_plugin
                    && (pane.id == self_plugin_id
                        || self_plugin_url.is_some_and(|plugin_url| {
                            pane.plugin_url.as_deref() == Some(plugin_url)
                        }));

                if is_leap_plugin || !pane.is_selectable {
                    return None;
                }

                let being_matched = !pane.is_suppressed || {
                    match self.config.suppressed_pane_behavior {
                        SuppressedPaneBehavior::Exclude => return None,
                        SuppressedPaneBehavior::DontMatch => false,
                        SuppressedPaneBehavior::Include => true,
                    }
                };

                Some(LeapTarget {
                    name: MatchedString::new(pane.title.clone()),
                    being_matched: Resettable::new(being_matched),
                    current: false,
                    location: LeapLocation::Pane {
                        pane_id: pane_id_from_pane(pane),
                        is_floating: pane.is_floating,
                        is_suppressed: pane.is_suppressed,
                    },
                })
            })
            .collect()
    }

    fn session_targets<'a, 'b>(
        &self,
        live_sessions: impl Iterator<Item = &'a SessionInfo>,
        resurrectable_sessions: impl Iterator<Item = &'b (String, Duration)>,
        match_current: bool,
    ) -> Vec<LeapTarget> {
        struct SessionTargetInfo {
            name: SessionName,
            is_current: bool,
            is_alive: bool,
        }

        let session_targets = live_sessions
            .map(|session| SessionTargetInfo {
                name: SessionName(session.name.clone()),
                is_current: session.is_current_session,
                is_alive: true,
            })
            .chain(resurrectable_sessions.map(|(name, _)| SessionTargetInfo {
                name: SessionName(name.clone()),
                is_current: false,
                is_alive: false,
            }));

        session_targets
            .map(|session| LeapTarget {
                name: MatchedString::new(session.name.0.clone()),
                being_matched: Resettable::new(!session.is_current || match_current),
                current: session.is_current,
                location: LeapLocation::Session {
                    session_name: session.name,
                    is_alive: session.is_alive,
                },
            })
            .collect()
    }

    fn assumed_selection(&self) -> Option<usize> {
        self.manual_selection
            .or_else(|| self.targets.iter().position(|target| target.current))
            .or(Some(0))
            .filter(|index| *index < self.targets.len())
    }

    fn refresh_pane_focus(&mut self) {
        let plugin_id = get_plugin_ids().plugin_id;
        let is_pane_focused = self.last_panes.as_ref().is_some_and(|pane_manifest| {
            pane_manifest
                .panes
                .values()
                .flatten()
                .any(|pane| pane.is_plugin && pane.id == plugin_id && pane.is_focused)
        });

        if self.is_pane_focused && !is_pane_focused {
            match self.config.pane_unfocus_behavior {
                PaneUnfocusBehavior::None => (),
                PaneUnfocusBehavior::Close => close_self(),
            }
        }

        self.is_pane_focused = is_pane_focused;
    }

    fn reset_matching(&mut self) -> bool {
        let mut did_reset = false;

        self.manual_selection = None;
        for target in self.targets.iter_mut() {
            did_reset = did_reset || !matches!(target.name.state(), MatchingState::Pending);
            target.being_matched.reset();
            target.name.reset();
        }

        did_reset
    }
}
