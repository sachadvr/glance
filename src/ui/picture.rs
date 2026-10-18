use std::io::Cursor;

use image::ImageReader;
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{FilterType, Resize, StatefulImage};

/// An image, drawn through the terminal's graphics protocol (kitty, iTerm2, sixel) or in half blocks without one.
pub struct Picture {
    protocol: StatefulProtocol,
}

impl Picture {
    /// `None` when the bytes are no image glance decodes (PNG, JPEG, GIF, WebP) or a broken one.
    // The bytes pick the decoder, not the name: a JPEG saved as .png still shows.
    pub fn decode(bytes: &[u8], picker: &Picker) -> Option<Picture> {
        let image = ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok()?.decode().ok()?;
        Some(Picture { protocol: picker.new_resize_protocol(image) })
    }
}

// Shrunk to fit, never enlarged: an icon stays an icon. Nearest, the default, would shred a screenshot's text.
pub fn draw_picture(frame: &mut Frame, area: Rect, picture: &mut Picture) {
    frame.render_stateful_widget(StatefulImage::default().resize(Resize::Fit(Some(FilterType::Triangle))), area, &mut picture.protocol);
}

#[cfg(test)]
mod tests {
    use image::{ImageFormat, Rgb, RgbImage};
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;

    use super::*;

    // Darkening downwards: a cell of one colour would be drawn blank, whatever the image.
    fn encoded(format: ImageFormat) -> Vec<u8> {
        let mut bytes = Vec::new();
        RgbImage::from_fn(40, 40, |_, y| Rgb([255 - (6 * y) as u8, 0, 0])).write_to(&mut Cursor::new(&mut bytes), format).unwrap();
        bytes
    }

    #[test]
    fn decode_reads_png_jpeg_gif_and_webp() {
        for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Gif, ImageFormat::WebP] {
            assert!(Picture::decode(&encoded(format), &Picker::halfblocks()).is_some(), "{format:?}");
        }
    }

    #[test]
    fn decode_refuses_a_truncated_image_and_what_is_no_image() {
        let png = encoded(ImageFormat::Png);
        assert!(Picture::decode(&png[..png.len() / 2], &Picker::halfblocks()).is_none());
        assert!(Picture::decode(b"\x89PNG\r\n\x1a\nnot really", &Picker::halfblocks()).is_none());
        assert!(Picture::decode(b"plain text", &Picker::halfblocks()).is_none());
        assert!(Picture::decode(&[], &Picker::halfblocks()).is_none());
    }

    #[test]
    fn draw_picture_paints_half_blocks_in_the_colours_of_the_image() {
        let mut picture = Picture::decode(&encoded(ImageFormat::Png), &Picker::halfblocks()).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();

        terminal.draw(|frame| draw_picture(frame, frame.area(), &mut picture)).unwrap();

        let cell = &terminal.backend().buffer()[(0, 0)];
        let (Color::Rgb(upper, 0, 0), Color::Rgb(lower, 0, 0)) = (cell.fg, cell.bg) else {
            panic!("not the image's red: {cell:?}");
        };
        assert_eq!(cell.symbol(), "▀");
        assert!(upper > lower, "{upper} above {lower}");
    }
}
