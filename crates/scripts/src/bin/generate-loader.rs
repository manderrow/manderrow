#!/usr/bin/env -S rust-script -f
//! ```cargo
//! [dependencies]
//! sailfish = "0.9.0"
//! ```
use std::f64::consts::PI;
use std::io::Write;

use sailfish::{runtime::Render, TemplateSimple};

struct Point<T = f64> {
    x: T,
    y: T,
}

fn p<T>(x: T, y: T) -> Point<T> {
    Point { x, y }
}

fn calculate_light_clip(angle: f64, radius: f64) -> ([f64; 2], f64) {
    let circumference = 2.0 * PI * radius;

    // 3. First, 1/4 of circumfence of 90 degrees. To start from top of the view,
    //    we must rotate it by 90 degrees. By default circle will start on the right.
    //    Stroke offset effectively rotates the circle.
    // 4. Second, calculate dash array. We need dash array containing only two parts -
    //    visible dash, and invisible dash.
    //    Visible dash should have length of the chosen angle. Full circle is 360 degrees,
    //    and this 360 degrees does also equal the entire circumference. We want just a part of
    //    this entire circle to be visible - (angle / 360 degrees) returns a percentage value
    //    (between 0.0 and 1.0) of how much circumference should be visible.
    //    Hence, we then multiply (angle / 360) times the entire circumference.
    let stroke_offset = ((90.0 + angle) / 2.0 / 360.0) * circumference;
    let stroke_dasharray = (angle / 360.0) * circumference;

    (
        [stroke_dasharray, circumference - stroke_dasharray],
        stroke_offset,
    )
}

//M 25 1.5 a  1    1   0 0 0   0   47
//M 25 1.5 a 23.5 23.5 0 0 0 -23.5 23.5

struct KeySplines<'a>(&'a str);

impl Render for KeySplines<'_> {
    fn render(&self, b: &mut sailfish::runtime::Buffer) -> Result<(), sailfish::RenderError> {
        b.push_str(self.0);
        b.push_str(";");
        b.push_str(self.0);
        Ok(())
    }
}

#[derive(TemplateSimple)]
#[template(path = "light_channel.svg", escape = false, rm_whitespace = true)]
struct LightChannel<'a> {
    class: &'a str,
}

#[derive(Clone, Copy)]
struct LightProps {
    padding: f64,
    diameter: f64,
    radius: f64,
    dasharray: [f64; 2],
    dashoffset: f64,
    channel_multiplier: f64,
}

#[derive(TemplateSimple)]
#[template(path = "loader_tmpl.svg", escape = false, rm_whitespace = true)]
struct LoaderTemplate<'a> {
    canvas_size: f64,

    light: LightProps,

    padding: Point,
    hcanvas_size: f64,
    slider_length: f64,
    slider_pos_start: f64,
    slider_pos_end: f64,
    duration: f64,
    qduration: f64,
    key_splines: KeySplines<'a>,
    axis_thickness: f64,
    axis_start: f64,
    axis_end: f64,
    slider_thickness: f64,
    hslider_thickness: f64,
    handle_thickness: f64,
    slider_mid_start: f64,
    vertical_end: Point<[f64; 2]>,
    horizontal_end: Point<[f64; 2]>,
}

pub fn main() {
    let duration: f64 = 1.0;
    // let duration: f64 = 4.8;
    let qduration: f64 = duration / 4.0;

    let key_splines: &str = "0.364212423249 0 0.635787576751 1";

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

    let padding = p(
        (horizontal_outset - (slider_length / 2.0 - handle_thickness / 2.0)).max(0.0),
        (vertical_outset - (slider_length / 2.0 - handle_thickness / 2.0)).max(0.0)
    );

    let (vertical_end, horizontal_end) = {
        let handle_start = p::<f64>(slider_pos_start + slider_length / 2.0, hcanvas_size);
        let handle_end = p::<f64>(canvas_size - handle_start.x, handle_start.y);

        (
            p(
                [
                    handle_start.x - vertical_outset,
                    handle_end.x + vertical_outset,
                ],
                [
                    handle_start.y + vertical_outset,
                    handle_end.y - vertical_outset,
                ],
            ),
            p(
                [
                    handle_start.x - horizontal_outset,
                    handle_end.x + horizontal_outset,
                ],
                [
                    handle_start.y + horizontal_outset,
                    handle_end.y - horizontal_outset,
                ],
            ),
        )
    };

    let light_diameter = 36.0;
    let light_padding: f64 = (canvas_size - light_diameter) / 2.0;
    let light_radius = light_diameter / 2.0;

    let (light_dasharray, light_dashoffset) = calculate_light_clip(40.0, light_radius);

    let light = LightProps {
        padding: light_padding,
        diameter: light_diameter,
        radius: light_radius,
        dasharray: light_dasharray,
        dashoffset: light_dashoffset,
        channel_multiplier: 0.6,
    };

    std::io::stdout()
        .write_all(
            LoaderTemplate {
                canvas_size,
                light,
                padding,
                hcanvas_size,
                slider_length,
                slider_pos_start,
                slider_pos_end,
                duration,
                qduration,
                key_splines: KeySplines(key_splines),
                axis_thickness,
                axis_start: hcanvas_size - axis_length,
                axis_end: hcanvas_size + axis_length,
                slider_thickness,
                hslider_thickness: slider_thickness / 2.0,
                handle_thickness,
                slider_mid_start,
                vertical_end,
                horizontal_end,
            }
            .render_once()
            .unwrap()
            .as_bytes(),
        )
        .unwrap();
}
