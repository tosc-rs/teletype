#![cfg_attr(not(any(test, feature = "use-std")), no_std)]

use embedded_graphics::{
    mono_font::MonoTextStyle,
    prelude::{DrawTarget, Drawable, PixelColor, Point, Size},
    primitives::{PrimitiveStyleBuilder, Rectangle, StyledDrawable},
    text::Text,
};
use input_mgr::{RingLine, Source};

#[derive(Clone)]
pub struct ColorStyle<'font, ColorKind: PixelColor> {
    pub background: ColorKind,
    pub local_editing_font: MonoTextStyle<'font, ColorKind>,
    pub remote_editing_font: MonoTextStyle<'font, ColorKind>,
    pub local_history_font: MonoTextStyle<'font, ColorKind>,
    pub remote_history_font: MonoTextStyle<'font, ColorKind>,
    pub local_editing_background: ColorKind,
    pub remote_editing_background: ColorKind,
    pub local_history_background: ColorKind,
    pub remote_history_background: ColorKind,
    pub margin_chars: u32,
}

pub fn drawer_color<'font, ColorKind, Display, const WIDTH: usize, const HEIGHT: usize>(
    disp: &mut Display,
    rline: &RingLine<HEIGHT, WIDTH>,
    style: ColorStyle<'font, ColorKind>,
) -> Result<(), <Display as DrawTarget>::Error>
where
    ColorKind: PixelColor,
    Display: DrawTarget<Color = ColorKind>,
{
    let full_display = disp.bounding_box();

    let local_edit_char_pixels_y = style.local_editing_font.font.character_size.height;
    let remote_edit_char_pixels_y = style.remote_editing_font.font.character_size.height;
    let local_hist_char_pixels_y = style.local_history_font.font.character_size.height;
    let remote_hist_char_pixels_y = style.remote_history_font.font.character_size.height;

    // Blank the background
    let mut y_idx: u32 = full_display.size.height;
    let x_width = full_display.size.width;
    disp.fill_solid(&full_display, style.background)?;

    let left_margin_px = if style.margin_chars != 0 {
        let local_edit_char_pixels_x = style.local_editing_font.font.character_size.width
            + style.local_editing_font.font.character_spacing;
        let remote_edit_char_pixels_x = style.remote_editing_font.font.character_size.width
            + style.remote_editing_font.font.character_spacing;
        let local_hist_char_pixels_x = style.local_history_font.font.character_size.width
            + style.local_history_font.font.character_spacing;
        let remote_hist_char_pixels_x = style.remote_history_font.font.character_size.width
            + style.remote_history_font.font.character_spacing;
        let largest_width = [
            local_edit_char_pixels_x,
            remote_edit_char_pixels_x,
            local_hist_char_pixels_x,
            remote_hist_char_pixels_x,
        ]
        .into_iter()
        .max()
        .unwrap_or(0);

        largest_width * style.margin_chars
    } else {
        0
    };

    let width_margin = if style.margin_chars != 0 {
        x_width - (2 * left_margin_px)
    } else {
        x_width
    };

    let local_edit_bkgd_style = PrimitiveStyleBuilder::new()
        .fill_color(style.local_editing_background)
        .build();
    for line in rline.iter_local_editing() {
        y_idx -= local_edit_char_pixels_y;
        let bar = Rectangle::new(
            Point {
                x: left_margin_px as i32,
                y: y_idx as i32,
            },
            Size {
                width: width_margin,
                height: local_edit_char_pixels_y,
            },
        );
        bar.draw_styled(&local_edit_bkgd_style, disp)?;

        Text::new(
            line.as_str(),
            Point {
                x: left_margin_px as i32,
                y: (y_idx + style.local_editing_font.font.baseline) as i32,
            },
            style.local_editing_font,
        )
        .draw(disp)?;
    }

    let remote_edit_bkgd_style = PrimitiveStyleBuilder::new()
        .fill_color(style.remote_editing_background)
        .build();
    for line in rline.iter_remote_editing() {
        y_idx -= remote_edit_char_pixels_y;
        let bar = Rectangle::new(
            Point {
                x: left_margin_px as i32,
                y: y_idx as i32,
            },
            Size {
                width: width_margin,
                height: remote_edit_char_pixels_y,
            },
        );
        bar.draw_styled(&remote_edit_bkgd_style, disp)?;

        Text::new(
            line.as_str(),
            Point {
                x: left_margin_px as i32,
                y: (y_idx + style.remote_editing_font.font.baseline) as i32,
            },
            style.remote_editing_font,
        )
        .draw(disp)?;
    }

    let local_hist_bkgd_style = PrimitiveStyleBuilder::new()
        .fill_color(style.local_history_background)
        .build();
    let remote_hist_bkgd_style = PrimitiveStyleBuilder::new()
        .fill_color(style.remote_history_background)
        .build();
    for line in rline.iter_history() {
        let (line_y, font, bkgd) = match line.status() {
            Source::Local => (
                local_hist_char_pixels_y,
                style.local_history_font,
                local_hist_bkgd_style,
            ),
            Source::Remote => (
                remote_hist_char_pixels_y,
                style.remote_history_font,
                remote_hist_bkgd_style,
            ),
        };

        y_idx -= line_y;
        let bar = Rectangle::new(
            Point {
                x: left_margin_px as i32,
                y: y_idx as i32,
            },
            Size {
                width: width_margin,
                height: line_y,
            },
        );
        bar.draw_styled(&bkgd, disp)?;

        Text::new(
            line.as_str(),
            Point {
                x: left_margin_px as i32,
                y: (y_idx + font.font.baseline) as i32,
            },
            font,
        )
        .draw(disp)?;
    }

    Ok(())
}

#[derive(Clone)]
pub struct BwStyle<'font, ColorKind: PixelColor> {
    pub background: ColorKind,
    pub font: MonoTextStyle<'font, ColorKind>,
}

pub fn drawer_bw<'font, ColorKind, Display, const WIDTH: usize, const HEIGHT: usize>(
    disp: &mut Display,
    rline: &RingLine<HEIGHT, WIDTH>,
    style: BwStyle<'font, ColorKind>,
) -> Result<(), <Display as DrawTarget>::Error>
where
    ColorKind: PixelColor,
    Display: DrawTarget<Color = ColorKind>,
{
    let full_display = disp.bounding_box();
    let char_pixels_y = style.font.font.character_size.height;
    let char_pixels_x = style.font.font.character_size.width + style.font.font.character_spacing;

    // Blank the background
    let mut y_idx: u32 = full_display.size.height;
    let x_width = full_display.size.width;
    let l_gutter = 2 * char_pixels_x;
    let r_gutter = x_width - (2 * char_pixels_x);
    disp.fill_solid(&full_display, style.background)?;

    for line in rline.iter_local_editing() {
        // Bail once we run out of screen
        y_idx = match y_idx.checked_sub(char_pixels_y) {
            Some(y) => y,
            None => return Ok(()),
        };

        let font_y = (y_idx + style.font.font.baseline) as i32;

        // Left gutter
        let lgpt = Point { x: 0, y: font_y };
        Text::new("> ", lgpt, style.font).draw(disp)?;

        // Text
        let ltpt = Point {
            x: l_gutter as i32,
            y: font_y,
        };
        Text::new(line.as_str(), ltpt, style.font).draw(disp)?;

        // Right gutter
        let rgpt = Point {
            x: r_gutter as i32,
            y: font_y,
        };
        Text::new(" #", rgpt, style.font).draw(disp)?;
    }

    for line in rline.iter_remote_editing() {
        // Bail once we run out of screen
        y_idx = match y_idx.checked_sub(char_pixels_y) {
            Some(y) => y,
            None => return Ok(()),
        };

        let font_y = (y_idx + style.font.font.baseline) as i32;

        // Left gutter
        let lgpt = Point { x: 0, y: font_y };
        Text::new("< ", lgpt, style.font).draw(disp)?;

        // Text
        let ltpt = Point {
            x: l_gutter as i32,
            y: font_y,
        };
        Text::new(line.as_str(), ltpt, style.font).draw(disp)?;

        // Right gutter
        let rgpt = Point {
            x: r_gutter as i32,
            y: font_y,
        };
        Text::new(" #", rgpt, style.font).draw(disp)?;
    }

    // let local_hist_bkgd_style = PrimitiveStyleBuilder::new().fill_color(style.local_history_background).build();
    // let remote_hist_bkgd_style = PrimitiveStyleBuilder::new().fill_color(style.remote_history_background).build();
    for line in rline.iter_history() {
        // Bail once we run out of screen
        y_idx = match y_idx.checked_sub(char_pixels_y) {
            Some(y) => y,
            None => return Ok(()),
        };
        let (lgutter, rgutter) = match line.status() {
            Source::Local => (">|", "|>"),
            Source::Remote => ("<|", "|<"),
        } ;

        let font_y = (y_idx + style.font.font.baseline) as i32;

        // Left gutter
        let lgpt = Point { x: 0, y: font_y };
        Text::new(lgutter, lgpt, style.font).draw(disp)?;

        // Text
        let ltpt = Point {
            x: l_gutter as i32,
            y: font_y,
        };
        Text::new(line.as_str(), ltpt, style.font).draw(disp)?;

        // Right gutter
        let rgpt = Point {
            x: r_gutter as i32,
            y: font_y,
        };
        Text::new(rgutter, rgpt, style.font).draw(disp)?;
    }

    Ok(())
}
