use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AuthorStat {
    pub author: String,
    pub lines: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Stats {
    pub total_files: u64,
    pub total_lines: u64,
    pub authors: Vec<AuthorStat>,
}

impl Stats {
    pub fn empty() -> Self {
        Self {
            total_files: 0,
            total_lines: 0,
            authors: Vec::new(),
        }
    }

    pub fn from_counts(counts: HashMap<String, u64>, total_files: u64) -> Self {
        let total_lines: u64 = counts.values().sum();

        let mut authors: Vec<AuthorStat> = counts
            .into_iter()
            .map(|(author, lines)| {
                let percentage = if total_lines > 0 {
                    let raw = (lines as f64 / total_lines as f64 * 100.0) * 100.0;
                    raw.round() / 100.0
                } else {
                    0.0
                };
                AuthorStat {
                    author,
                    lines,
                    percentage,
                }
            })
            .collect();

        authors.sort_by(|left, right| {
            right
                .lines
                .cmp(&left.lines)
                .then_with(|| left.author.cmp(&right.author))
        });

        Self {
            total_files,
            total_lines,
            authors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_and_sorts() {
        let mut counts = HashMap::new();
        counts.insert("Bob".to_owned(), 30);
        counts.insert("Alice".to_owned(), 70);
        counts.insert("Carol".to_owned(), 0);

        let stats = Stats::from_counts(counts, 3);
        assert_eq!(stats.total_files, 3);
        assert_eq!(stats.total_lines, 100);
        assert_eq!(stats.authors[0].author, "Alice");
        assert_eq!(stats.authors[0].percentage, 70.0);
        assert_eq!(stats.authors[1].author, "Bob");
        assert_eq!(stats.authors[1].percentage, 30.0);
        assert_eq!(stats.authors[2].author, "Carol");
        assert_eq!(stats.authors[2].percentage, 0.0);
    }

    #[test]
    fn ties_sort_by_author() {
        let mut counts = HashMap::new();
        counts.insert("Zed".to_owned(), 5);
        counts.insert("Amy".to_owned(), 5);
        let stats = Stats::from_counts(counts, 2);
        assert_eq!(stats.authors[0].author, "Amy");
        assert_eq!(stats.authors[1].author, "Zed");
    }

    #[test]
    fn zero_lines() {
        let stats = Stats::from_counts(HashMap::new(), 2);
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.total_lines, 0);
        assert!(stats.authors.is_empty());
    }
}
