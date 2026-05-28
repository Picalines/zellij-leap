pub struct MatchedString {
    string: String,
    state: MatchingState,
}

#[derive(Default)]
pub enum MatchingState {
    #[default]
    Pending,
    Anchors {
        anchors: Vec<(usize, char)>,
    },
    Found {
        start: usize,
        len: usize,
    },
    None,
}

pub enum MatchingPart {
    String,
    Anchor,
    Match,
}

impl MatchedString {
    pub fn new(string: String) -> Self {
        Self {
            string,
            state: MatchingState::default(),
        }
    }

    pub fn str(&self) -> &str {
        &self.string
    }

    pub fn state(&self) -> &MatchingState {
        &self.state
    }

    pub fn parts(&self) -> MatchingParts<'_> {
        MatchingParts {
            string: &self.string,
            state: &self.state,
            cursor: 0,
            step: 0,
        }
    }

    pub fn match_char(&mut self, ch: char) -> bool {
        match self.state {
            MatchingState::None => false,
            MatchingState::Pending => {
                let anchors: Vec<(usize, Option<char>)> = self
                    .string
                    .to_ascii_lowercase()
                    .match_indices(ch.to_ascii_lowercase())
                    .map(|(i, _)| (i, self.string.chars().nth(i + 1)))
                    .collect();

                let (has_moved, next_state) = match &anchors[..] {
                    [] => (false, MatchingState::None),
                    [(end_index, _)] => (
                        true,
                        MatchingState::Found {
                            start: *end_index,
                            len: 1,
                        },
                    ),
                    _ => (
                        true,
                        MatchingState::Anchors {
                            anchors: anchors
                                .iter()
                                .filter_map(|(i, next_char)| {
                                    next_char.and_then(|c| Some((*i, c.to_ascii_lowercase())))
                                })
                                .collect(),
                        },
                    ),
                };

                self.state = next_state;
                has_moved
            }
            MatchingState::Anchors { ref anchors } => {
                let ch_lower = ch.to_ascii_lowercase();
                let matched_anchor = anchors.iter().find(|(_, next_char)| *next_char == ch_lower);

                match matched_anchor {
                    None => false,
                    Some((start, _)) => {
                        self.state = MatchingState::Found {
                            start: *start,
                            len: 2,
                        };
                        true
                    }
                }
            }
            MatchingState::Found { start, len } => {
                if start + len == self.string.len() {
                    return false;
                }

                let (_, from_start) = self.string.split_at(start);
                let (_, after_match) = from_start.split_at(len);

                let matches = after_match.starts_with(ch.to_ascii_lowercase())
                    || after_match.starts_with(ch.to_ascii_uppercase());

                if matches {
                    self.state = MatchingState::Found {
                        start,
                        len: len + 1,
                    };
                }

                matches
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = MatchingState::default();
    }
}

pub struct MatchingParts<'a> {
    string: &'a str,
    state: &'a MatchingState,
    cursor: usize,
    step: usize,
}

impl<'a> Iterator for MatchingParts<'a> {
    type Item = (MatchingPart, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            MatchingState::Pending | MatchingState::None => {
                if self.step > 0 {
                    return None;
                }

                self.step = 1;
                Some((MatchingPart::String, self.string))
            }
            MatchingState::Anchors { anchors } => loop {
                if self.step < anchors.len() {
                    let anchor_start = anchors[self.step].0;

                    if self.cursor < anchor_start {
                        let part = &self.string[self.cursor..anchor_start];
                        self.cursor = anchor_start;
                        return Some((MatchingPart::String, part));
                    }

                    self.step += 1;
                    self.cursor = self.next_char_end(anchor_start);

                    return Some((
                        MatchingPart::Anchor,
                        &self.string[anchor_start..self.cursor],
                    ));
                } else if self.cursor < self.string.len() {
                    let part = &self.string[self.cursor..];
                    self.cursor = self.string.len();
                    return Some((MatchingPart::String, part));
                } else {
                    return None;
                }
            },
            MatchingState::Found { start, len } => {
                let start = *start;
                let end = start.saturating_add(*len).min(self.string.len());

                while self.step < 3 {
                    let part = match self.step {
                        0 => (MatchingPart::String, &self.string[..start]),
                        1 => (MatchingPart::Match, &self.string[start..end]),
                        _ => (MatchingPart::String, &self.string[end..]),
                    };

                    self.step += 1;

                    if !part.1.is_empty() || self.string.is_empty() {
                        return Some(part);
                    }
                }

                None
            }
        }
    }
}

impl<'a> MatchingParts<'a> {
    fn next_char_end(&self, start: usize) -> usize {
        self.string[start..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| start + i)
            .unwrap_or(self.string.len())
    }
}
