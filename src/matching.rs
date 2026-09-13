pub struct MatchedString {
    string: String,
    state: MatchingState,
}

#[derive(Default)]
enum MatchingState {
    #[default]
    Pending,
    Anchors {
        anchors: Vec<(usize, char)>,
    },
    Found {
        start: usize,
        len: usize,
        end_reached: bool,
    },
    None,
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
                let anchors: Vec<(usize, usize, Option<char>)> = self
                    .string
                    .char_indices()
                    .filter(|(_, fch)| case_insensitive_equal(*fch, ch))
                    .map(|(start, fch)| {
                        let end = start + fch.len_utf8();
                        (start, end, self.string[end..].chars().next())
                    })
                    .collect();

                let (has_moved, next_state) = match &anchors[..] {
                    [] => (false, MatchingState::None),
                    [(start, end, _)] => (
                        true,
                        MatchingState::Found {
                            start: *start,
                            len: end - start,
                            end_reached: *end == self.string.len(),
                        },
                    ),
                    _ => (
                        true,
                        MatchingState::Anchors {
                            anchors: anchors
                                .iter()
                                .filter_map(|(start, _, next_char)| {
                                    next_char.map(|next_char| (*start, next_char))
                                })
                                .collect(),
                        },
                    ),
                };

                self.state = next_state;
                has_moved
            }
            MatchingState::Anchors { ref anchors } => {
                let matched_anchor = anchors
                    .iter()
                    .find(|(_, next_char)| case_insensitive_equal(*next_char, ch));

                match matched_anchor {
                    None => false,
                    Some((start, next_char)) => {
                        let first_char_len = self.string[*start..]
                            .chars()
                            .next()
                            .map(char::len_utf8)
                            .unwrap_or(0);
                        let len = first_char_len + next_char.len_utf8();
                        self.state = MatchingState::Found {
                            start: *start,
                            len,
                            end_reached: *start + len == self.string.len(),
                        };
                        true
                    }
                }
            }
            MatchingState::Found {
                start,
                len,
                end_reached,
            } => {
                if end_reached {
                    return false;
                }

                let after_match = &self.string[start + len..];
                let next_char = after_match.chars().next().unwrap();
                let matches = case_insensitive_equal(next_char, ch);

                if matches {
                    let len = len + next_char.len_utf8();
                    self.state = MatchingState::Found {
                        start,
                        len,
                        end_reached: start + len == self.string.len(),
                    };
                }

                matches
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = MatchingState::default();
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.state, MatchingState::Pending)
    }

    pub fn is_end_reached(&self) -> bool {
        matches!(
            self.state,
            MatchingState::Found {
                end_reached: true,
                ..
            }
        )
    }
}

fn case_insensitive_equal(left: char, right: char) -> bool {
    left.to_lowercase().eq(right.to_lowercase())
}

pub enum MatchingPart {
    String,
    Anchor,
    Match,
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
            MatchingState::Anchors { anchors } => {
                if self.step < anchors.len() {
                    let anchor_start = anchors[self.step].0;

                    if self.cursor < anchor_start {
                        let part = &self.string[self.cursor..anchor_start];
                        self.cursor = anchor_start;
                        return Some((MatchingPart::String, part));
                    }

                    self.step += 1;
                    self.cursor = self.next_char_end(anchor_start);

                    Some((
                        MatchingPart::Anchor,
                        &self.string[anchor_start..self.cursor],
                    ))
                } else if self.cursor < self.string.len() {
                    let part = &self.string[self.cursor..];
                    self.cursor = self.string.len();
                    Some((MatchingPart::String, part))
                } else {
                    None
                }
            }
            MatchingState::Found { start, len, .. } => {
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
