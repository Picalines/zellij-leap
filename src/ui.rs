use owo_colors::OwoColorize;

use crate::{config::*, matching::*, state::*};

pub fn render(state: &LeapState, rows: usize, cols: usize) {
    let hint_text = match state.error {
        Some(_) => "error:",
        None => match state.targets.len() {
            0 => match state.config.target {
                LeapTargetKind::Tab | LeapTargetKind::TabExceptActive => "awaiting tabs...",
                LeapTargetKind::PaneInActiveTab => "awaiting panes...",
                LeapTargetKind::Session | LeapTargetKind::SessionExceptCurrent => {
                    "awaiting sessions..."
                }
            },
            _ => match state.config.target {
                LeapTargetKind::Tab | LeapTargetKind::TabExceptActive => "leap to tab:",
                LeapTargetKind::PaneInActiveTab => "leap to pane:",
                LeapTargetKind::Session | LeapTargetKind::SessionExceptCurrent => {
                    "leap to session:"
                }
            },
        },
    };

    // I wanted this code to not allocate, so beware: we calculate size of UI before rendering
    // it. This is kinda required, because it's hard to calculate a width of Styled strings

    // (1 for hint_text)
    debug_assert_eq!(hint_text.lines().count(), 1);
    let height = 1 + match state.error {
        None => state.targets.len(),
        Some(ref error) => {
            debug_assert_eq!(error.lines().count(), 1);
            1
        }
    };

    let prefix_width = 2;
    let width = text_width(hint_text)
        .max(
            state
                .error
                .as_ref()
                .map(|text| text_width(text))
                .unwrap_or(0),
        )
        .max(
            state
                .targets
                .iter()
                .map(|target| {
                    prefix_width
                        + text_width(target.name.str())
                        + render_target_detail(target).map(text_width).unwrap_or(0)
                })
                .max()
                .unwrap_or(0),
        );

    let print_left_padding = start_centered_render(rows, cols, height, width);

    println!("{}", hint_text.dimmed());

    if let Some(ref error_text) = state.error {
        print_left_padding();
        print!("{}", error_text.red());
        return;
    }

    let selection_index = state.assumed_selection();

    for (target_index, target) in state.targets.iter().enumerate() {
        print_left_padding();

        let prefix = match (target.current, selection_index == Some(target_index)) {
            (true, true) => "» ".green().into_styled(),
            (false, true) => "> ".green().into_styled(),
            (true, false) => "- ".dimmed().into_styled(),
            _ => "  ".white().into_styled(),
        };
        debug_assert_eq!(text_width(prefix.inner()), prefix_width);
        print!("{}", prefix);

        let detail = render_target_detail(target).unwrap_or("");

        if !target.being_matched.current {
            println!(
                "{}{}",
                target.name.str().dimmed().strikethrough(),
                detail.dimmed().strikethrough()
            );
            continue;
        }

        if matches!(target.name.state(), MatchingState::Pending) {
            println!("{}{}", target.name.str(), detail.dimmed());
            continue;
        }

        for (part_index, (part_kind, part)) in target.name.parts().enumerate() {
            match part_kind {
                MatchingPart::String if part_index == 0 => print!("{}", part.dimmed()),
                MatchingPart::String => {
                    let first_char_len = part.chars().next().map(char::len_utf8).unwrap_or(0);
                    let (first_char, rest) = part.split_at(first_char_len);
                    print!("{}{}", first_char, rest.dimmed());
                }
                MatchingPart::Anchor => print!("{}", part.yellow()),
                MatchingPart::Match => print!("{}", part.green()),
            }
        }

        println!("{}", detail.dimmed());
    }
}

fn render_target_detail(target: &LeapTarget) -> Option<&str> {
    let target_detail = match target.location {
        LeapLocation::Pane {
            is_suppressed,
            is_floating,
            ..
        } => match (is_suppressed, is_floating) {
            (true, true) => " (suppressed) (floating)",
            (true, false) => " (suppressed)",
            (false, true) => " (floating)",
            (false, false) => return None,
        },
        LeapLocation::Session {
            is_alive: false, ..
        } => " (resurrect)",
        _ => return None,
    };

    debug_assert!(target_detail.starts_with(" "));

    Some(target_detail)
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
