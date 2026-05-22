mod leap_config;
mod matching;
mod utils;

use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use std::time::Duration;
use zellij_tile::prelude::*;

use crate::leap_config::*;
use crate::matching::*;
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
    error: Option<String>,
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
        let hint_text = match self.error {
            Some(_) => "error:",
            None => match self.targets.len() {
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
            },
        };

        // I wanted this code to not allocate, so beware:
        // we calculate size of UI before rendering it

        // (1 for hint_text)
        let height = 1 + match self.error {
            None => self.targets.len(),
            Some(_) => 1,
        };

        let target_prefix_width = 2;
        let width = Self::text_width(hint_text)
            .max(
                self.error
                    .as_ref()
                    .map(|text| Self::text_width(text))
                    .unwrap_or(0),
            )
            .max(
                self.targets
                    .iter()
                    .map(|target| target_prefix_width + Self::text_width(target.name.str()))
                    .max()
                    .unwrap_or(0),
            );

        let print_left_padding = Self::start_centered_render(rows, cols, height, width);

        println!("{}", hint_text.dimmed());

        if let Some(ref error_text) = self.error {
            print_left_padding();
            print!("{}", error_text.red());
            return;
        }

        for target in self.targets.iter() {
            print_left_padding();

            let prefix = if target.current { "> " } else { "  " };
            debug_assert_eq!(prefix.len(), target_prefix_width);
            print!("{}", prefix.green());

            if !target.being_matched {
                println!("{}", target.name.str().dimmed().strikethrough());
                continue;
            }

            if matches!(target.name.state(), MatchingState::Pending) {
                println!("{}", target.name.str());
                continue;
            }

            for (i, (part_kind, part)) in target.name.parts().enumerate() {
                match part_kind {
                    MatchingPart::String if i == 0 => print!("{}", part.dimmed()),
                    MatchingPart::String => {
                        let (first_char, rest) = part.split_at(1);
                        print!("{}{}", first_char, rest.dimmed());
                    }
                    MatchingPart::Anchor => print!("{}", part.yellow()),
                    MatchingPart::Match => print!("{}", part.green()),
                }
            }

            println!();
        }
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
            LeapTargetKind::Session => match get_session_list() {
                Err(error) => {
                    self.error = Some(format!("failed to fetch session list: {}", error));
                    true
                }
                Ok(sessions) => {
                    self.assign_session_targets(
                        sessions.live_sessions.iter(),
                        sessions.resurrectable_sessions.iter(),
                    );
                    true
                }
            },
            _ => false,
        }
    }

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
        if self.error.is_some() {
            return;
        }

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
            .any(|target| !matches!(target.name.state(), MatchingState::Pending))
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

    fn text_width(str: &str) -> usize {
        str.chars().count()
    }

    fn start_centered_render(
        total_rows: usize,
        total_cols: usize,
        used_rows: usize,
        used_cols: usize,
    ) -> impl Fn() {
        let top_padding = total_rows.saturating_sub(used_rows).saturating_div(2);
        print!("{:\n<1$}", "", top_padding);

        let left_padding = total_cols.saturating_sub(used_cols).saturating_div(2);
        let print_left_padding = move || {
            print!("{: <1$}", "", left_padding);
        };

        print_left_padding();

        print_left_padding
    }
}
