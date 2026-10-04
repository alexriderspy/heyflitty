//! The model ends each reply with `[POINT:#id:label]` (a listed UI element),
//! `[POINT:x,y:label:screenN]` (pixels, for things not in the list) or `[POINT:none]`.
//! This parses that tag and holds it back from the spoken text while streaming.

use std::sync::OnceLock;

use regex::Regex;

#[derive(Debug, PartialEq)]
pub enum PointTarget {
    /// An element id from the UI Automation list.
    Element(usize),
    /// Pixels in the screenshot the model saw; screen_number is 1-based, None means the cursor's screen.
    Pixels { x: f64, y: f64, screen_number: Option<usize> },
}

#[derive(Debug, PartialEq)]
pub struct PointTag {
    pub target: PointTarget,
    pub label: String,
}

fn tag_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"\[POINT:(?:none|#(\d+)(?::([^\]]*?))?|(\d+)\s*,\s*(\d+)(?::([^\]:]*?))?(?::screen(\d+))?)\]\s*$").expect("valid pattern")
    })
}

/// Splits a finished reply into the text to show and the optional point target.
pub fn parse(reply: &str) -> (String, Option<PointTag>) {
    let Some(captures) = tag_pattern().captures(reply) else { return (reply.trim().to_string(), None) };
    let spoken = reply[..captures.get(0).expect("whole match").start()].trim().to_string();
    let text = |index: usize| captures.get(index).map(|found| found.as_str().trim().to_string());
    let number = |index: usize| captures.get(index).map(|found| found.as_str().parse::<usize>().expect("pattern only matches digits"));
    let tag = if let Some(id) = number(1) {
        Some(PointTag { target: PointTarget::Element(id), label: text(2).unwrap_or_default() })
    } else if let (Some(x), Some(y)) = (number(3), number(4)) {
        Some(PointTag { target: PointTarget::Pixels { x: x as f64, y: y as f64, screen_number: number(6) }, label: text(5).unwrap_or_default() })
    } else {
        None
    };
    (spoken, tag)
}

/// Turns streamed text into whole sentences to speak, never speaking the point tag.
pub struct SentenceSplitter {
    buffer: String,
    reached_tag: bool,
}

impl SentenceSplitter {
    pub fn new() -> Self {
        Self { buffer: String::new(), reached_tag: false }
    }

    /// Feeds a text delta and returns any sentences that are now complete.
    pub fn push(&mut self, delta: &str) -> Vec<String> {
        if self.reached_tag {
            return Vec::new();
        }
        self.buffer.push_str(delta);
        if let Some(tag_start) = self.buffer.find("[POINT") {
            self.buffer.truncate(tag_start);
            self.reached_tag = true;
        } else if let Some(bracket) = self.buffer.rfind('[') {
            // A partial "[POI" at the end may become a tag: only release text before it.
            if "[POINT".starts_with(&self.buffer[bracket..]) {
                let held_back = self.buffer.split_off(bracket);
                let sentences = self.take_complete_sentences();
                self.buffer.push_str(&held_back);
                return sentences;
            }
        }
        self.take_complete_sentences()
    }

    /// Returns whatever is left once the stream ends.
    pub fn finish(&mut self) -> Option<String> {
        if let Some(tag_start) = self.buffer.find('[') {
            self.buffer.truncate(tag_start);
        }
        let rest = std::mem::take(&mut self.buffer).trim().to_string();
        (!rest.is_empty()).then_some(rest)
    }

    fn take_complete_sentences(&mut self) -> Vec<String> {
        let mut sentences = Vec::new();
        loop {
            let boundary = self.buffer.char_indices().find(|&(index, character)| {
                matches!(character, '.' | '!' | '?')
                    && self.buffer[index + character.len_utf8()..].starts_with(char::is_whitespace)
            });
            let Some((index, character)) = boundary else { break };
            let sentence: String = self.buffer.drain(..index + character.len_utf8()).collect();
            let sentence = sentence.trim().to_string();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
        }
        sentences
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_point_with_label_and_screen() {
        let (spoken, tag) = parse("open the color inspector up top. [POINT:1100,42:color inspector:screen2]");
        assert_eq!(spoken, "open the color inspector up top.");
        assert_eq!(
            tag,
            Some(PointTag { target: PointTarget::Pixels { x: 1100.0, y: 42.0, screen_number: Some(2) }, label: "color inspector".into() })
        );
    }

    #[test]
    fn parses_point_at_element_id() {
        let (spoken, tag) = parse("click personalization on the left. [POINT:#14:Personalization]");
        assert_eq!(spoken, "click personalization on the left.");
        assert_eq!(tag, Some(PointTag { target: PointTarget::Element(14), label: "Personalization".into() }));
    }

    #[test]
    fn point_none_means_no_target() {
        assert_eq!(parse("html is the skeleton of a page. [POINT:none]"), ("html is the skeleton of a page.".into(), None));
    }

    #[test]
    fn splitter_never_speaks_the_tag_even_when_split_across_deltas() {
        let mut splitter = SentenceSplitter::new();
        let mut spoken = Vec::new();
        for delta in ["see the menu up top? ", "click it and hit commit. [PO", "INT:285,11:source control]"] {
            spoken.extend(splitter.push(delta));
        }
        spoken.extend(splitter.finish());
        assert_eq!(spoken, vec!["see the menu up top?", "click it and hit commit."]);
    }

    #[test]
    fn splitter_flushes_trailing_text_without_punctuation() {
        let mut splitter = SentenceSplitter::new();
        assert!(splitter.push("one more thing").is_empty());
        assert_eq!(splitter.finish(), Some("one more thing".into()));
    }
}
