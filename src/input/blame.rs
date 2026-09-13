use crate::domain::BlameLine;

use super::log::is_hash;
use super::Document;

pub(super) fn detect(lines: &[&str]) -> Option<Document> {
    parse_blame(lines).map(Document::Blame)
}

// "^7bfd4f45 (Linus 2024-08-21 23:03:02 +0200 12) code": `git blame`'s default format. `-C` or a rename
// adds the original path between the hash and the parenthesis; a boundary commit is marked with "^".
fn blame_line(line: &str) -> Option<BlameLine> {
    let (hash, rest) = line.split_once(' ')?;
    let hash = hash.trim_start_matches('^');
    if !is_hash(hash) {
        return None;
    }
    let (_, annotated) = rest.split_once('(')?;
    let (annotation, code) = annotated.split_once(')')?;
    let mut fields = annotation.split_whitespace().rev();
    let number = fields.next()?.parse().ok()?;
    let _zone = fields.next().filter(|zone| zone.starts_with(['+', '-']))?;
    let _time = fields.next().filter(|time| time.contains(':'))?;
    let date = fields.next().filter(|date| date.len() == 10 && date.as_bytes()[4] == b'-')?;
    let author: Vec<&str> = fields.collect();

    Some(BlameLine {
        hash: hash.chars().take(8).collect(),
        author: author.into_iter().rev().collect::<Vec<_>>().join(" "),
        date: date.to_string(),
        number,
        code: code.strip_prefix(' ').unwrap_or(code).to_string(),
    })
}

fn parse_blame(lines: &[&str]) -> Option<Vec<BlameLine>> {
    let blame = lines.iter().filter(|line| !line.is_empty()).map(|line| blame_line(line)).collect::<Option<Vec<BlameLine>>>()?;

    (!blame.is_empty()).then_some(blame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_blame_reads_boundary_commits_paths_and_empty_lines() {
        let blame = "^7bfd4f45 (Linus 2024-08-21 23:03:02 +0200 1) <?php\n7bfd4f45 (Linus 2024-08-21 23:03:02 +0200 2) \n238d6155 src/Old.php (Grace Hopper 2026-10-16 18:01:20 +0200 3)     return $a (b);\n00000000 (Not Committed Yet 2026-10-18 10:00:00 +0200 4) new\n";

        let lines = parse_blame(&blame.lines().collect::<Vec<_>>()).unwrap();

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], BlameLine { hash: "7bfd4f45".into(), author: "Linus".into(), date: "2024-08-21".into(), number: 1, code: "<?php".into() });
        assert_eq!(lines[1].code, "");
        assert_eq!((lines[2].author.as_str(), lines[2].number, lines[2].code.as_str()), ("Grace Hopper", 3, "    return $a (b);"));
        assert!(lines[2].is_committed() && !lines[3].is_committed());
    }

    #[test]
    fn blame_line_refuses_an_annotation_without_a_date_and_time() {
        assert_eq!(blame_line("7bfd4f45 (Linus 12) code"), None);
        assert_eq!(blame_line("7bfd4f45 (Linus 21/08/2024 23:03:02 +0200 12) code"), None);
        assert_eq!(blame_line("7bfd4f45 no annotation"), None);
    }
}
