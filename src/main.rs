mod matched_string;

use matched_string::MatchedString;
use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

struct LeapTarget {
    tab_id: usize,
    name: MatchedString,
    being_matched: bool,
}

#[derive(Default)]
struct LeapState {
    targets: Vec<LeapTarget>,
    input: String,
}

register_plugin!(LeapState);

// TODO: support for jumping to panes
// TODO: add option to skip current tab
// TODO: handle exact matches or tabs with same names

impl ZellijPlugin for LeapState {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        subscribe(&[EventType::Key, EventType::TabUpdate, EventType::Visible]);

        rename_plugin_pane(get_plugin_ids().plugin_id, "leap");
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::TabUpdate(tabs) => self.handle_tab_update(tabs),
            Event::Visible(true) => {
                self.reset_input();
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let hint_text = "jump to tab:";
        let mut lines = vec![hint_text.italic().dimmed().to_string()];
        let mut width = hint_text.len();

        for target in self.targets.iter() {
            width = width.max(target.name.str().chars().count());

            if !target.being_matched {
                lines.push(format!("{}", target.name.str().dimmed().strikethrough()));
                continue;
            }

            let (before_match, matched, after_match) = target.name.split();

            lines.push(if matched.is_empty() {
                target.name.str().to_string()
            } else if after_match.is_empty() {
                format!(
                    "{}{}{}",
                    before_match.dimmed(),
                    matched.bold().dimmed(),
                    after_match
                )
            } else {
                let (next_input, after_next) = after_match.split_at(1);
                format!(
                    "{}{}{}{}",
                    before_match.dimmed(),
                    matched.bold().dimmed(),
                    next_input.underline().green(),
                    after_next.dimmed()
                )
            });
        }

        let top_padding = rows.saturating_sub(lines.len()).saturating_div(2);
        let left_padding = cols.saturating_sub(width).saturating_div(2);

        print!("{:\n<1$}", "", top_padding);
        for line in lines {
            println!("{: <1$}{line}", "", left_padding);
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
        if let Some(active_tab) = tabs.iter().find(|tab| tab.active)
            && !active_tab.are_floating_panes_visible
        {
            close_self();
            return false;
        }

        self.targets = tabs
            .iter()
            .map(|tab| LeapTarget {
                tab_id: tab.tab_id,
                name: MatchedString::new(tab.name.clone()),
                being_matched: true,
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
                last_matched_tab_id = Some(target.tab_id);
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
            (1, Some(tab_id)) => {
                switch_tab_to(tab_id as u32 + 1);
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

        // TODO: add configuration that also hides floating panes
        close_self();
        false
    }
}
