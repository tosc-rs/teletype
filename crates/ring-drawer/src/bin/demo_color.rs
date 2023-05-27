use std::time::{Duration, Instant};

use embedded_graphics::{prelude::{Dimensions, Point, Size, DrawTarget, IntoStorage, RgbColor, WebColors}, primitives::Rectangle, Pixel, pixelcolor::Rgb888, mono_font::MonoTextStyle};
use input_mgr::RingLine;
use minifb::{WindowOptions, Scale, Window};
use profont::PROFONT_12_POINT;

const DISP_PIXELS_X: usize = 400;
const DISP_PIXELS_Y: usize = 240;
const DISP_DEFAULT: [u32; DISP_PIXELS_X * DISP_PIXELS_Y] = [0; DISP_PIXELS_X * DISP_PIXELS_Y];

struct Display {
    pixels: [u32; DISP_PIXELS_X * DISP_PIXELS_Y],
}

impl Default for Display {
    fn default() -> Self {
        Self {
            pixels: DISP_DEFAULT,
        }
    }
}

impl Dimensions for Display {
    fn bounding_box(&self) -> Rectangle {
        Rectangle {
            top_left: Point::new(0, 0),
            size: Size::new(DISP_PIXELS_X as u32, DISP_PIXELS_Y as u32),
        }
    }
}


impl DrawTarget for Display {
    type Color = Rgb888;

    type Error = ();

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(pt, col) in pixels.into_iter() {
            let idx = (pt.y.unsigned_abs() * DISP_PIXELS_X as u32) + pt.x.unsigned_abs();
            let idx = idx as usize;
            if let Some(pix) = self.pixels.get_mut(idx) {
                *pix = col.into_storage();
            }
        }
        Ok(())
    }
}


fn main() {
    let mut disp = Display::default();
    let mut options = WindowOptions::default();
    options.scale = Scale::X4;
    let mut window =
        Window::new("Test - ESC to exit", DISP_PIXELS_X, DISP_PIXELS_Y, options).unwrap();
    window.limit_update_rate(Some(Duration::from_micros(1_000_000 / 60)));

    let style = ring_drawer::ColorStyle {
        background: Rgb888::BLACK,
        local_editing_font: MonoTextStyle::new(&PROFONT_12_POINT, Rgb888::WHITE),
        remote_editing_font: MonoTextStyle::new(&PROFONT_12_POINT, Rgb888::WHITE),
        local_history_font: MonoTextStyle::new(&PROFONT_12_POINT, Rgb888::BLACK),
        remote_history_font: MonoTextStyle::new(&PROFONT_12_POINT, Rgb888::BLACK),
        local_editing_background: Rgb888::CSS_DARK_BLUE,
        remote_editing_background: Rgb888::CSS_DARK_GREEN,
        local_history_background: Rgb888::CSS_LIGHT_BLUE,
        remote_history_background: Rgb888::CSS_LIGHT_GREEN,
        margin_chars: 1,
    };

    let mut rline = RingLine::<16, 48>::new();

    let mut timer = Instant::now();
    let mut ctr = -1;
    loop {
        if timer.elapsed() > Duration::from_millis(100) {
            timer = Instant::now();
            ctr += 1;
            match ctr {
                0..=8 => rline.append_local_char(b'$').unwrap(),
                9 => rline.submit_local_editing(),
                10..=18 => rline.append_remote_char(b'#').unwrap(),
                19 => rline.submit_remote_editing(),
                _ => ctr = -1,
            }
        }
        ring_drawer::drawer_color(&mut disp, &rline, style.clone()).unwrap();
        window
            .update_with_buffer(&disp.pixels, DISP_PIXELS_X, DISP_PIXELS_Y)
            .unwrap();
    }
}
