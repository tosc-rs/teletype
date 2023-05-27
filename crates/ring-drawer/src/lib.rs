// impl<C: PixelColor> StyledDrawable<PrimitiveStyle<C>> for Rectangle {
//     type Color = C;
//     type Output = ();

//     fn draw_styled<D>(
//         &self,
//         style: &PrimitiveStyle<C>,
//         target: &mut D,
//     ) -> Result<Self::Output, D::Error>
//     where
//         D: DrawTarget<Color = C>,
//     {

use embedded_graphics::{
    mono_font::MonoTextStyle,
    prelude::{Drawable, DrawTarget, PixelColor, Point, Size}, primitives::{PrimitiveStyleBuilder, Rectangle, StyledDrawable}, text::Text,
};
use input_mgr::{RingLine, Source};

#[derive(Clone)]
pub struct Style<'font, ColorKind: PixelColor> {
    pub background: ColorKind,
    pub local_editing_font: MonoTextStyle<'font, ColorKind>,
    pub remote_editing_font: MonoTextStyle<'font, ColorKind>,
    pub local_history_font: MonoTextStyle<'font, ColorKind>,
    pub remote_history_font: MonoTextStyle<'font, ColorKind>,
    pub local_editing_background: ColorKind,
    pub remote_editing_background: ColorKind,
    pub local_history_background: ColorKind,
    pub remote_history_background: ColorKind,
}

pub fn drawer<'font, ColorKind, Display, const WIDTH: usize, const HEIGHT: usize>(
    disp: &mut Display,
    rline: &RingLine<HEIGHT, WIDTH>,
    style: Style<'font, ColorKind>,
) -> Result<(), <Display as DrawTarget>::Error>
where
    ColorKind: PixelColor,
    Display: DrawTarget<Color = ColorKind>,
{
    let full_display = disp.bounding_box();

    let local_edit_char_pixels_x = style.local_editing_font.font.character_size.width
        + style.local_editing_font.font.character_spacing;
    let local_edit_char_pixels_y = style.local_editing_font.font.character_size.height;
    let remote_edit_char_pixels_x = style.remote_editing_font.font.character_size.width
        + style.remote_editing_font.font.character_spacing;
    let remote_edit_char_pixels_y = style.remote_editing_font.font.character_size.height;

    let local_hist_char_pixels_x = style.local_history_font.font.character_size.width
        + style.local_history_font.font.character_spacing;
    let local_hist_char_pixels_y = style.local_history_font.font.character_size.height;
    let remote_hist_char_pixels_x = style.remote_history_font.font.character_size.width
        + style.remote_history_font.font.character_spacing;
    let remote_hist_char_pixels_y = style.remote_history_font.font.character_size.height;

    // Blank the background
    let mut y_idx: u32 = full_display.size.height;
    let x_width = full_display.size.width;
    disp.fill_solid(&full_display, style.background)?;

    let local_edit_bkgd_style = PrimitiveStyleBuilder::new().fill_color(style.local_editing_background).build();
    for line in rline.iter_local_editing() {
        y_idx -= local_edit_char_pixels_y;
        let bar = Rectangle::new(
            Point {
                x: 0,
                y: y_idx as i32,
            },
            Size {
                width: x_width,
                height: local_edit_char_pixels_y,
            },
        );
        bar.draw_styled(&local_edit_bkgd_style, disp)?;

        Text::new(
            line.as_str(),
            Point {
                x: 0,
                y: (y_idx + style.local_editing_font.font.baseline) as i32,
            },
            style.local_editing_font,
        )
        .draw(disp)?;
    }

    let remote_edit_bkgd_style = PrimitiveStyleBuilder::new().fill_color(style.remote_editing_background).build();
    for line in rline.iter_remote_editing() {
        y_idx -= remote_edit_char_pixels_y;
        let bar = Rectangle::new(
            Point {
                x: 0,
                y: y_idx as i32,
            },
            Size {
                width: x_width,
                height: remote_edit_char_pixels_y,
            },
        );
        bar.draw_styled(&remote_edit_bkgd_style, disp)?;

        Text::new(
            line.as_str(),
            Point {
                x: 0,
                y: (y_idx + style.remote_editing_font.font.baseline) as i32,
            },
            style.remote_editing_font,
        )
        .draw(disp)?;
    }

    let local_hist_bkgd_style = PrimitiveStyleBuilder::new().fill_color(style.local_history_background).build();
    let remote_hist_bkgd_style = PrimitiveStyleBuilder::new().fill_color(style.remote_history_background).build();
    for line in rline.iter_history() {
        let (line_y, font, bkgd) = match line.status() {
            Source::Local => (local_hist_char_pixels_y, style.local_history_font, local_hist_bkgd_style),
            Source::Remote => (remote_hist_char_pixels_y, style.remote_history_font, remote_hist_bkgd_style),
        } ;


        y_idx -= line_y;
        let bar = Rectangle::new(
            Point {
                x: 0,
                y: y_idx as i32,
            },
            Size {
                width: x_width,
                height: line_y,
            },
        );
        bar.draw_styled(&bkgd, disp)?;

        Text::new(
            line.as_str(),
            Point {
                x: 0,
                y: (y_idx + font.font.baseline) as i32,
            },
            font,
        )
        .draw(disp)?;
    }

    Ok(())
}
