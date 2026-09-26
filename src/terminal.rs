//! The escape sequences glance writes to its terminal, beside what ratatui draws.

use std::io::{self, Write};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Asks the terminal to put `text` on the clipboard. OSC 52 travels over the terminal's own connection,
/// so it reaches the local clipboard over SSH too.
pub fn copy_to_clipboard(text: &str) -> io::Result<()> {
    write_sequence(&osc52(text))
}

/// OSC 22 sets the mouse pointer's shape (Ghostty, kitty, foot); a terminal without it ignores the sequence.
pub fn set_pointer(is_over_target: bool) -> io::Result<()> {
    write_sequence(&osc22(is_over_target))
}

fn write_sequence(sequence: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    write!(stdout, "{sequence}")?;
    stdout.flush()
}

fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

fn osc22(is_over_target: bool) -> String {
    let shape = if is_over_target { "pointer" } else { "default" };
    format!("\x1b]22;{shape}\x1b\\")
}

// Standard alphabet with padding, as terminals expect: no URL-safe variant, no line breaks.
fn base64(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let byte = |index: usize| u32::from(chunk.get(index).copied().unwrap_or(0));
        let group = (byte(0) << 16) | (byte(1) << 8) | byte(2);
        // A chunk of n bytes carries n + 1 sextets; the rest of the quartet is padding.
        for sextet in 0..4 {
            if sextet <= chunk.len() {
                let index = (group >> (18 - 6 * sextet)) & 0x3f;
                encoded.push(char::from(ALPHABET[index as usize]));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_encodes_the_utf8_bytes_of_multibyte_text() {
        assert_eq!(base64("é€".as_bytes()), "w6nigqw=");
    }

    #[test]
    fn base64_uses_the_standard_alphabet_for_the_last_two_sextets() {
        assert_eq!(base64(&[0xfb, 0xff]), "+/8=");
    }

    #[test]
    fn osc52_wraps_the_base64_payload_in_the_clipboard_sequence() {
        assert_eq!(osc52("f"), "\x1b]52;c;Zg==\x07");
    }

    #[test]
    fn osc22_names_the_pointer_shape_over_a_target_and_the_default_elsewhere() {
        assert_eq!(osc22(true), "\x1b]22;pointer\x1b\\");
        assert_eq!(osc22(false), "\x1b]22;default\x1b\\");
    }
}
