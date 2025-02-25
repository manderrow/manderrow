#!/usr/bin/env -S rust-script -f
use std::io::Write;
use std::ops::Add;

struct Point<T = f64> {
    x: T,
    y: T,
}

impl<T: Add> Add for Point<T> {
    type Output = Point<T::Output>;

    fn add(self, other: Self) -> Self::Output {
        p(self.x + other.x, self.y + other.y)
    }
}

fn p<T>(x: T, y: T) -> Point<T> {
    Point { x, y }
}

pub fn main() {
    let duration: f64 = 1.6;
    // let duration: f64 = 4.8;
    let qduration: f64 = duration / 4.0;

    let key_splines: &str = "0.364212423249 0 0.635787576751 1";
    let key_splines_dup: String = format!("{key_splines}; {key_splines}");

    let canvas_size: f64 = 50.0;
    let hcanvas_size = canvas_size / 2.0;

    let axis_thickness: f64 = 6.0;
    let axis_length: f64 = 22.0 - axis_thickness / 2.0;
    let slider_thickness: f64 = 6.0;
    let handle_thickness: f64 = 8.0;
    let slider_mid_start = hcanvas_size - slider_thickness / 2.0;

    let slider_length: f64 = 16.0;

    let slider_bound = 3.0;

    let slider_pos_start: f64 = slider_bound;
    let slider_pos_end: f64 = canvas_size - slider_bound - slider_length;

    // let horizontal_outset: f64 = 20.0;
    let horizontal_outset: f64 = 0.0;
    let vertical_outset: f64 = 0.0;

    let padding_x: f64 = (horizontal_outset - (slider_length / 2.0 - handle_thickness / 2.0)).max(0.0);
    let padding_y: f64 = (vertical_outset - (slider_length / 2.0 - handle_thickness / 2.0)).max(0.0);

    let (vertical_end, horizontal_end) = {
        let handle_start = p::<f64>(slider_pos_start + slider_length / 2.0, hcanvas_size);
        let handle_end = p::<f64>(canvas_size - handle_start.x, handle_start.y);

        (p(
            [handle_start.x - vertical_outset, handle_end.x + vertical_outset],
            [handle_start.y + vertical_outset, handle_end.y - vertical_outset],
        ), p(
            [handle_start.x - horizontal_outset, handle_end.x + horizontal_outset],
            [handle_start.y + horizontal_outset, handle_end.y - horizontal_outset],
        ))
    };

    write!(
        std::io::stdout(),
        include_str!("loader_tmpl.svg"),
        canvas_width = canvas_size + padding_x * 2.0,
        canvas_height = canvas_size + padding_y * 2.0,
        padding_x = padding_x,
        padding_y = padding_y,
        hcanvas_size = hcanvas_size,
        slider_length = slider_length,
        slider_pos_start = slider_pos_start,
        slider_pos_end = slider_pos_end,
        duration = duration,
        qduration = qduration,
        key_splines = key_splines_dup,
        axis_thickness = axis_thickness,
        axis_start = hcanvas_size - axis_length,
        axis_end = hcanvas_size + axis_length,
        slider_thickness = slider_thickness,
        hslider_thickness = slider_thickness / 2.0,
        handle_thickness = handle_thickness,
        slider_mid_start = slider_mid_start,
        s_end_start_x = vertical_end.x[0],
        s_end_end_x = vertical_end.x[1],
        s_end_start_y = vertical_end.y[0],
        s_end_end_y = vertical_end.y[1],
        l_end_start_x = horizontal_end.x[0],
        l_end_end_x = horizontal_end.x[1],
        l_end_start_y = horizontal_end.y[0],
        l_end_end_y = horizontal_end.y[1]
    )
    .unwrap()
}
