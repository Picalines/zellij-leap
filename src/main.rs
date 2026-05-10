mod leap_config;
mod matched_string;

use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

use crate::leap_config::{EscapeBehavior, LeapConfig};
use crate::matched_string::MatchedString;

struct LeapTarget {
    tab_position: usize,
    name: MatchedString,
    being_matched: bool,
    current: bool,
}

#[derive(Default)]
struct LeapState {
    config: LeapConfig,
    targets: Vec<LeapTarget>,
    input: String,
    is_pane_focused: bool,
}

register_plugin!(LeapState);

// TODO: support for jumping to panes
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
            EventType::TabUpdate,
            EventType::Visible,
        ]);

        rename_plugin_pane(get_plugin_ids().plugin_id, "leap");
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::PaneUpdate(_) => {
                self.handle_pane_update();
                false
            }
            Event::TabUpdate(tabs) => self.handle_tab_update(tabs),
            Event::Visible(true) => {
                self.reset_input();
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let hint_text = "leap to tab:";

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
    fn reset_input(&mut self) {
        self.input.clear();

        for target in self.targets.iter_mut() {
            target.being_matched = true;
            target.name.reset();
        }
    }

    fn handle_tab_update(&mut self, tabs: Vec<TabInfo>) -> bool {
        self.input.clear();

        self.targets = tabs
            .iter()
            .map(|tab| LeapTarget {
                tab_position: tab.position,
                name: MatchedString::new(tab.name.clone()),
                being_matched: !tab.active || self.config.include_current_target,
                current: tab.active,
            })
            .collect();

        true
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        let has_ctrl = key.has_modifiers(&[KeyModifier::Ctrl]);
        let no_mods = key.key_modifiers.is_empty();

        match key.bare_key {
            BareKey::Esc => self.handle_escape(),
            BareKey::Char('u') if has_ctrl => {
                self.reset_input();
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
        let mut last_matched_tab_id: Option<usize> = None;

        for target in self.targets.iter_mut() {
            if !target.being_matched {
                continue;
            }

            if target.name.match_char(ch) {
                number_of_matches += 1;
                last_matched_tab_id = Some(target.tab_position);
            } else {
                target.being_matched = false;
            }
        }

        match (number_of_matches, last_matched_tab_id) {
            (0, _) => {
                // TODO: add option for "no matches" behavior
                // - just close
                // - hide floating panes
                // - reset and display message
                close_self();
            }
            (1, Some(tab_position)) => {
                switch_tab_to(tab_position as u32 + 1);
                _ = hide_floating_panes(None);
                close_self();
            }
            _ => {
                self.input.push(ch);
            }
        }
    }

    fn handle_escape(&mut self) -> bool {
        if !self.input.is_empty() {
            self.reset_input();
            return true;
        }

        match self.config.escape_behavior {
            EscapeBehavior::Close => close_self(),
            EscapeBehavior::HideFloatingPanes => _ = hide_floating_panes(None),
        }

        false
    }

    fn handle_pane_update(&mut self) {
        let Ok((_, focused_pane_id)) = get_focused_pane_info() else {
            return;
        };

        let plugin_id = get_plugin_ids().plugin_id;
        let is_focused = focused_pane_id == PaneId::Plugin(plugin_id);

        if self.is_pane_focused && !is_focused && self.config.close_on_pane_unfocus {
            close_self();
        }

        self.is_pane_focused = is_focused
    }
}
