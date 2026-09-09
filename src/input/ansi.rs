// Git colours what it sends a pager; CSI sequences and OSC hyperlinks are both dropped.
pub fn strip_ansi(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            plain.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(next) = chars.next() {
                    if next == '\x07' {
                        break;
                    }
                    if next == '\x1b' {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_drops_colours_and_hyperlinks() {
        assert_eq!(strip_ansi("\x1b[1;32m+ok\x1b[m \x1b]8;;http://x\x1b\\link\x1b]8;;\x1b\\"), "+ok link");
    }

    #[test]
    fn strip_ansi_ends_a_hyperlink_at_a_bell_too() {
        assert_eq!(strip_ansi("\x1b]8;;http://x\x07link\x1b]8;;\x07!"), "link!");
    }

    #[test]
    fn strip_ansi_drops_a_lone_escape_and_the_character_after_it() {
        assert_eq!(strip_ansi("a\x1b(b"), "ab");
    }
}
