mod leap_config;
mod matched_string;
mod utils;

use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use std::time::Duration;
use zellij_tile::prelude::*;

use crate::leap_config::*;
use crate::matched_string::MatchedString;
use crate::utils::*;

#[derive(Clone)]
enum LeapLocation {
    Tab(TabIndex),
    Pane(PaneId),
    Session(SessionName),
}

struct LeapTarget {
    name: MatchedString,
    being_matched: bool,
    current: bool,
    location: LeapLocation,
}

#[derive(Default)]
struct LeapState {
    config: LeapConfig,
    targets: Vec<LeapTarget>,
    is_pane_focused: bool,
}

register_plugin!(LeapState);

// TODO: handle exact matches or tabs with same names

impl ZellijPlugin for LeapState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config = LeapConfig::parse(configuration);

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        subscribe(&[
            EventType::Key,
            EventType::PaneUpdate,
            EventType::SessionUpdate,
            EventType::TabUpdate,
        ]);

        rename_plugin_pane(get_plugin_ids().plugin_id, "leap");
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::PaneUpdate(panes) => self.handle_pane_update(panes),
            Event::SessionUpdate(sessions, resurrectable_sessions) => {
                self.handle_session_update(sessions, resurrectable_sessions)
            }
            Event::TabUpdate(tabs) => self.handle_tab_update(tabs),
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let hint_text = match self.targets.len() {
            0 => match self.config.target {
                LeapTargetKind::Tab | LeapTargetKind::TabExceptActive => "awaiting tabs...",
                LeapTargetKind::PaneInActiveTab => "awaiting panes...",
                LeapTargetKind::Session => "awaiting sessions...",
            },
            _ => match self.config.target {
                LeapTargetKind::Tab | LeapTargetKind::TabExceptActive => "leap to tab:",
                LeapTargetKind::PaneInActiveTab => "leap to pane:",
                LeapTargetKind::Session => "leap to session:",
            },
        };

        // I wanted this code to not allocate, so beware:
        // we calculate size of UI before rendering it
        let target_prefix_width = 2;
        let width = hint_text.len().max(
            self.targets
                .iter()
                .map(|target| target_prefix_width + target.name.str().chars().count())
                .max()
                .unwrap_or(0),
        );
        let left_padding = cols.saturating_sub(width).saturating_div(2);

        let height = self.targets.len() + 1; // 1 for hint
        let top_padding = rows.saturating_sub(height).saturating_div(2);
        print!("{:\n<1$}", "", top_padding);

        print!("{: <1$}", "", left_padding);
        println!("{}", hint_text.dimmed());

        for target in self.targets.iter() {
            print!("{: <1$}", "", left_padding);

            let prefix = if target.current { "> " } else { "  " };
            debug_assert_eq!(prefix.len(), target_prefix_width);
            print!("{}", prefix.green());

            if !target.being_matched {
                println!("{}", target.name.str().dimmed().strikethrough());
                continue;
            }

            let (before_match, matched, after_match) = target.name.split();

            if matched.is_empty() {
                println!("{}", target.name.str());
            } else if after_match.is_empty() {
                println!(
                    "{}{}{}",
                    before_match.dimmed(),
                    matched.bold().dimmed(),
                    after_match
                )
            } else {
                let (next_input, after_next) = after_match.split_at(1);
                println!(
                    "{}{}{}{}",
                    before_match.dimmed(),
                    matched.bold().dimmed(),
                    next_input.underline().green(),
                    after_next.dimmed()
                )
            }
        }
    }
}

impl LeapState {
    fn handle_tab_update(&mut self, tabs: Vec<TabInfo>) -> bool {
        if self.is_pane_focused && !self.targets.is_empty() {
            return false;
        }

        match self.config.target {
            LeapTargetKind::Tab => {
                self.assign_tab_targets(tabs.iter(), true);
                true
            }
            LeapTargetKind::TabExceptActive => {
                self.assign_tab_targets(tabs.iter(), false);
                true
            }
            _ => false,
        }
    }

    fn handle_pane_update(&mut self, panes: PaneManifest) -> bool {
        let Some((focused_tab_index, _)) = self.handle_focus_state() else {
            return false;
        };

        if self.is_pane_focused && !self.targets.is_empty() {
            return false;
        }

        match self.config.target {
            LeapTargetKind::PaneInActiveTab => {
                let Some(panes) = panes.panes.get(&focused_tab_index.0) else {
                    return false;
                };

                self.assign_pane_targets(panes.iter());
                true
            }
            _ => false,
        }
    }

    fn handle_session_update(
        &mut self,
        live_sessions: Vec<SessionInfo>,
        resurrectable_sessions: Vec<(String, Duration)>,
    ) -> bool {
        if !self.targets.is_empty() {
            return false;
        }

        match self.config.target {
            LeapTargetKind::Session => {
                self.assign_session_targets(live_sessions.iter(), resurrectable_sessions.iter());
                true
            }
            _ => false,
        }
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        let has_ctrl = key.has_modifiers(&[KeyModifier::Ctrl]);
        let no_mods = key.key_modifiers.is_empty();

        match key.bare_key {
            BareKey::Esc => self.handle_escape(),
            BareKey::Char('u') if has_ctrl => {
                self.reset_matching();
                true
            }
            BareKey::Char(ch) if no_mods => {
                self.handle_char_key(ch);
                true
            }
            _ => false,
        }
    }

    fn handle_char_key(&mut self, ch: char) {
        let mut number_of_matches = 0usize;
        let mut last_matched_location: Option<LeapLocation> = None;

        for target in self.targets.iter_mut() {
            if !target.being_matched {
                continue;
            }

            if target.name.match_char(ch) {
                number_of_matches += 1;
                last_matched_location = Some(target.location.clone());
            } else {
                target.being_matched = false;
            }
        }

        match (number_of_matches, last_matched_location) {
            (0, _) => self.handle_no_matches(),
            (1, Some(leap_location)) => self.switch_to_location(&leap_location),
            _ => (),
        };
    }

    fn switch_to_location(&mut self, leap_location: &LeapLocation) {
        self.handle_matched();

        match leap_location {
            LeapLocation::Tab(tab_index) => switch_tab_to((*tab_index).0 as u32 + 1),
            LeapLocation::Pane(pane_id) => focus_pane_with_id(*pane_id, false, false),
            LeapLocation::Session(session_name) => switch_session(Some(&session_name.0)),
        }
    }

    fn handle_matched(&mut self) {
        // TODO: matched behavior option?
        self.targets.clear();
        _ = hide_floating_panes(None);
        close_self();
    }

    fn handle_no_matches(&mut self) {
        // TODO: add option for "no matches" behavior
        // - just close
        // - hide floating panes
        // - reset and display message
        close_self();
    }

    fn handle_escape(&mut self) -> bool {
        if self
            .targets
            .iter()
            .any(|target| target.name.matched().is_some())
        {
            self.reset_matching();
            return true;
        }

        match self.config.escape_behavior {
            EscapeBehavior::Close => close_self(),
            EscapeBehavior::HideFloatingPanes => _ = hide_floating_panes(None),
        }

        false
    }

    fn assign_tab_targets<'a>(
        &mut self,
        tabs: impl Iterator<Item = &'a TabInfo>,
        include_active: bool,
    ) {
        self.targets = tabs
            .map(|tab| LeapTarget {
                name: MatchedString::new(tab.name.clone()),
                being_matched: !tab.active || include_active,
                current: tab.active,
                location: LeapLocation::Tab(TabIndex(tab.position)),
            })
            .collect();
    }

    fn assign_pane_targets<'a>(&mut self, panes: impl Iterator<Item = &'a PaneInfo>) {
        let self_plugin_id = get_plugin_ids().plugin_id;

        self.targets = panes
            .filter_map(|pane| {
                let is_self_plugin = pane.is_plugin && pane.id == self_plugin_id;

                // TODO: config for suppressed panes?
                if is_self_plugin || !pane.is_selectable || pane.is_suppressed {
                    return None;
                }

                Some(LeapTarget {
                    name: MatchedString::new(pane.title.clone()),
                    being_matched: true,
                    current: false,
                    location: LeapLocation::Pane(pane_id_from_pane(pane)),
                })
            })
            .collect();
    }

    fn assign_session_targets<'a, 'b>(
        &mut self,
        live_sessions: impl Iterator<Item = &'a SessionInfo>,
        resurrectable_sessions: impl Iterator<Item = &'b (String, Duration)>,
    ) {
        struct SessionTargetInfo {
            name: SessionName,
            is_current: bool,
        }

        let session_targets = live_sessions
            .map(|session| SessionTargetInfo {
                name: SessionName(session.name.clone()),
                is_current: session.is_current_session,
            })
            .chain(resurrectable_sessions.map(|(name, _)| SessionTargetInfo {
                name: SessionName(name.clone()),
                is_current: false,
            }));

        self.targets = session_targets
            .map(|session| LeapTarget {
                name: MatchedString::new(session.name.0.clone()),
                being_matched: true,
                current: session.is_current,
                location: LeapLocation::Session(session.name),
            })
            .collect();
    }

    fn handle_focus_state(&mut self) -> Option<(TabIndex, PaneId)> {
        let (tab_index, focused_pane_id) = get_focused_pane_info().ok()?;

        let plugin_id = get_plugin_ids().plugin_id;
        let is_focused = focused_pane_id == PaneId::Plugin(plugin_id);

        if self.is_pane_focused && !is_focused {
            match self.config.pane_unfocus_behaviour {
                PaneUnfocusBehaviour::None => (),
                PaneUnfocusBehaviour::Close => close_self(),
            }
        }

        self.is_pane_focused = is_focused;

        Some((TabIndex(tab_index), focused_pane_id))
    }

    fn reset_matching(&mut self) {
        for target in self.targets.iter_mut() {
            target.being_matched = true;
            target.name.reset();
        }
    }
}
