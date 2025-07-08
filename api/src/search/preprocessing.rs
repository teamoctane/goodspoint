const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is", "it",
    "its", "of", "on", "that", "the", "to", "was", "will", "with", "the", "this", "but", "they",
    "have", "had", "what", "said", "each", "which", "she", "do", "how", "their", "if", "up", "out",
    "many", "then", "them", "these", "so", "some", "her", "would", "make", "like", "into", "him",
    "time", "two", "more", "go", "no", "way", "could", "my", "than", "first", "been", "call",
    "who", "oil", "sit", "now", "find", "down", "day", "did", "get", "come", "made", "may", "part",
];

#[inline]
fn is_stopword(word: &str) -> bool {
    STOPWORDS.contains(&word)
}

pub fn preprocess_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .filter(|word| {
            !word.is_empty()
                && word.len() > 1
                && !is_stopword(word)
                && word.chars().any(|c| c.is_alphabetic())
        })
        .collect::<Vec<&str>>()
        .join(" ")
}

pub fn has_stopwords(text: &str) -> bool {
    text.to_lowercase()
        .split_whitespace()
        .any(|word| is_stopword(word))
}

#[inline]
pub fn normalize_punctuation(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{201A}' | '\u{201E}' | '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            '\u{2026}' => '.',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

#[inline]
pub fn extract_keywords(text: &str) -> Vec<String> {
    preprocess_text(text)
        .split_whitespace()
        .filter(|word| word.len() >= 3)
        .map(str::to_string)
        .collect()
}

pub fn create_search_variants(query: &str) -> Vec<String> {
    let mut variants = Vec::with_capacity(4);

    variants.push(query.to_string());

    let processed = preprocess_text(query);
    variants.push(processed);

    let normalized = normalize_punctuation(query);
    if normalized != query {
        variants.push(normalized);
    }

    let keywords = extract_keywords(query);
    if keywords.len() > 1 {
        variants.push(keywords.join(" "));
    }

    variants.sort_unstable_by(|a, b| b.len().cmp(&a.len()));
    variants.dedup();

    variants
}
