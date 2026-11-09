#[derive(Debug)]
pub struct Chunk {
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

pub fn split(text: &str) -> Vec<Chunk> {
    let lines: Vec<_> = text.lines().collect();
    if lines.iter().any(|line| line.len() > 8192) {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        let mut end = start;
        let mut bytes = 0;
        while end < lines.len() && end - start < 64 {
            let additional = lines[end].len() + usize::from(end > start);
            if bytes + additional > 8192 {
                break;
            }
            bytes += additional;
            end += 1;
        }
        if end == start {
            return Vec::new();
        }
        let text = lines[start..end].join("\n");
        if !text.trim().is_empty() {
            chunks.push(Chunk {
                start_line: start + 1,
                end_line: end,
                text,
            });
        }
        if chunks.len() > 512 {
            return Vec::new();
        }
        if end == lines.len() {
            break;
        }
        start = end.saturating_sub(8).max(start + 1);
    }
    chunks
}

pub fn query(text: &str) -> anyhow::Result<String> {
    anyhow::ensure!(
        !text.trim().is_empty() && text.len() <= 512,
        "Enter a query with up to 512 UTF-8 bytes"
    );
    let terms: std::collections::BTreeSet<_> = text
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|word| !word.is_empty())
        .collect();
    anyhow::ensure!(
        !terms.is_empty() && terms.len() <= 32,
        "Enter 1–32 search terms"
    );
    Ok(terms
        .into_iter()
        .map(|word| format!("\"{word}\""))
        .collect::<Vec<_>>()
        .join(" OR "))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retrieval_chunks_keep_line_citations_and_utf8() {
        let text = (1..=100)
            .map(|line| format!("Line {line}: résumé 🦀"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = split(&text);
        assert_eq!(result.len(), 2);
        assert_eq!((result[0].start_line, result[0].end_line), (1, 64));
        assert_eq!((result[1].start_line, result[1].end_line), (57, 100));
        assert!(result[1].text.starts_with("Line 57:"));
        assert!(split(&"x".repeat(8193)).is_empty());
        let boundary = split(&"é".repeat(4096));
        assert_eq!(boundary.len(), 1);
        assert_eq!(boundary[0].text.len(), 8192);
        assert_eq!((boundary[0].start_line, boundary[0].end_line), (1, 1));
    }
    #[test]
    fn retrieval_query_is_quoted_data_not_fts_syntax() {
        assert_eq!(
            query("name OR secret*").unwrap(),
            "\"OR\" OR \"name\" OR \"secret\""
        );
        assert!(query("\"*\"").is_err());
        assert!(query(&"x".repeat(513)).is_err());
    }
}
