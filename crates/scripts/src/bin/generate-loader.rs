use std::io::Write;

use euclid::{Point2D, Vector2D};
use sailfish::{TemplateSimple, runtime::Render};

struct CanvasSpace;

type Point<T = f64> = Point2D<T, CanvasSpace>;
type Vector<T = f64> = Vector2D<T, CanvasSpace>;
type Angle = euclid::Angle<f64>;

fn p<T>(x: T, y: T) -> Point<T> {
    Point::new(x, y)
}

fn v<T>(x: T, y: T) -> Vector<T> {
    Vector::new(x, y)
}

struct PathCoords<T>(T);

impl Render for PathCoords<Point> {
    fn render(&self, b: &mut sailfish::runtime::Buffer) -> Result<(), sailfish::RenderError> {
        self.0.x.render(b)?;
        b.push(' ');
        self.0.y.render(b)
    }
}

trait AsPathCoords: Sized {
    fn as_path_coords(self) -> PathCoords<Self>;
}

impl<T> AsPathCoords for T
where
    PathCoords<T>: Render,
{
    fn as_path_coords(self) -> PathCoords<Self> {
        PathCoords(self)
    }
}

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
    thickness: f64,
    radius: f64,
    channel_angle: Angle,
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

    enable_handle: bool,
    enable_handle_extra: bool,
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
        (vertical_outset - (slider_length / 2.0 - handle_thickness / 2.0)).max(0.0),
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

    let light = LightProps {
        radius: 17.0,
        thickness: 3.0,
        channel_angle: Angle::degrees(40.0),
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

                enable_handle: false,
                enable_handle_extra: false,
            }
            .render_once()
            .unwrap()
            .as_bytes(),
        )
        .unwrap();
}
