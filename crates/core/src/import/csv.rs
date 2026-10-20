//! A small RFC 4180 CSV reader: quoted fields, `""` escapes, embedded commas
//! and newlines, CRLF or LF line ends, and a leading UTF-8 BOM.

/// Every record as its fields (blank lines become a single empty field).
pub fn parse(text: &str) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    // True once the current record has any content (so a trailing newline
    // doesn't add an empty record).
    let mut started = false;
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = false,
                c => field.push(c),
            }
            continue;
        }
        match c {
            '"' => {
                quoted = true;
                started = true;
            }
            ',' => {
                row.push(std::mem::take(&mut field));
                started = true;
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                started = false;
            }
            c => {
                field.push(c);
                started = true;
            }
        }
    }
    if started || !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::parse;

    fn rows(text: &str) -> Vec<Vec<&'static str>> {
        parse(text)
            .into_iter()
            .map(|r| r.into_iter().map(|f| &*Box::leak(f.into_boxed_str())).collect())
            .collect()
    }

    #[test]
    fn plain_rows_with_crlf_and_lf() {
        assert_eq!(rows("a,b,c\r\n1,2,3\n"), vec![vec!["a", "b", "c"], vec!["1", "2", "3"]]);
    }

    #[test]
    fn quoted_commas_quotes_and_newlines() {
        let text = "TYPE,CONTENT\ntask,\"Buy milk, eggs\"\ntask,\"Say \"\"hi\"\"\"\ntask,\"line one\nline two\"\n";
        assert_eq!(
            rows(text),
            vec![
                vec!["TYPE", "CONTENT"],
                vec!["task", "Buy milk, eggs"],
                vec!["task", "Say \"hi\""],
                vec!["task", "line one\nline two"],
            ]
        );
    }

    #[test]
    fn bom_blank_lines_and_empty_fields() {
        let text = "\u{feff}a,b\r\n\r\n,x\r\nlast,";
        assert_eq!(rows(text), vec![vec!["a", "b"], vec![""], vec!["", "x"], vec!["last", ""]]);
    }

    #[test]
    fn empty_text_has_no_rows() {
        assert!(parse("").is_empty());
    }
}
