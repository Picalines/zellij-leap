pub struct MatchedString {
    string: String,
    match_start: Option<usize>,
    match_len: usize,
}

impl MatchedString {
    pub fn new(string: String) -> Self {
        Self {
            string,
            match_start: None,
            match_len: 0,
        }
    }

    pub fn str(&self) -> &str {
        &self.string
    }

    pub fn split(&self) -> (&str, &str, &str) {
        let (before_start, from_start) = self.string.split_at(self.match_start.unwrap_or(0));
        let (matched, after_matched) = from_start.split_at(self.match_len);
        (before_start, matched, after_matched)
    }

    pub fn matched(&self) -> Option<&str> {
        let (_, matched, _) = self.split();
        match matched {
            "" => None,
            _ => Some(matched),
        }
    }

    pub fn match_char(&mut self, ch: char) -> bool {
        let ch_lower = ch.to_ascii_lowercase();
        let ch_upper = ch.to_ascii_uppercase();

        match self.match_start {
            None => match self
                .string
                .find(|c: char| c.to_ascii_lowercase() == ch_lower)
            {
                None => false,
                Some(first_match) => {
                    self.match_start = Some(first_match);
                    self.match_len = 1;
                    true
                }
            },
            Some(_) => {
                let (_, _, after_matched) = self.split();

                if after_matched.starts_with(ch_lower) || after_matched.starts_with(ch_upper) {
                    self.match_len += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.match_start = None;
        self.match_len = 0;
    }
}
