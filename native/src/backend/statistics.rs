use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStatistics {
    pub words: usize,
    pub characters: usize,
    pub characters_without_spaces: usize,
}

pub fn count_text(text: &str) -> TextStatistics {
    TextStatistics {
        words: text.unicode_words().count(),
        characters: text.chars().count(),
        characters_without_spaces: text.chars().filter(|character| !character.is_whitespace()).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words_and_unicode_characters() {
        let statistics = count_text("Hello World\nCafé");
        assert_eq!(statistics.words, 3);
        assert_eq!(statistics.characters, 16);
        assert_eq!(statistics.characters_without_spaces, 14);
    }

    #[test]
    fn handles_empty_and_whitespace_documents() {
        assert_eq!(count_text("").words, 0);
        assert_eq!(count_text(" \t\n").characters_without_spaces, 0);
        assert_eq!(count_text("你好 世界").words, 4);
    }
}
