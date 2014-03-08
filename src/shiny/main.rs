#[feature(phase)];
extern crate cgmath;
extern crate color;
extern crate glutil;
#[phase(syntax, link)]
extern crate calx;

use std::cmp::max;
use glutil::app::App;
use glutil::app;
use glutil::key;
use cgmath::aabb::{Aabb, Aabb2};
use cgmath::point::{Point, Point2};
use cgmath::vector::{Vec2};
use color::rgb::{ToRGB, consts};
use calx::rectutil::RectUtil;
use calx::gen_id;

static PANGRAM: &'static str =
"how quickly daft jumping zebras vex. \
SPHINX OF BLACK QUARTZ: JUDGE MY VOW. \
12345 67890 !@#$%^ &*()_+-= []{};: \"'\\ \
,./ <>?";

enum Align {
    Left,
    Center,
    Right
}

fn print_words<C: ToRGB>(
    app: &mut App, area: &Aabb2<f32>, color: &C, align: Align, text: &str) {
    let words: ~[&str] = text.split(' ').collect();
    let bounds = words.map(|&w| app.string_bounds(w).dim().x as uint);
    let mut i = 0;
    let origin = area.min().add_v(&Vec2::new(0.0, app::FONT_HEIGHT)).to_vec();
    let width = area.dim().x;
    let max_lines = (area.dim().y / app::FONT_HEIGHT) as uint;
    let mut pos = origin;
    let mut line = 0;
    while i < words.len() && line < max_lines {
        let (n, len) = num_fitting_words(width as uint, app::FONT_SPACE as uint, bounds.slice(i, bounds.len()));
        let n = max(1, n);

        let diff = area.dim().x - len as f32;
        match align {
            Left => (),
            Center => { pos.x += diff / 2.0; },
            Right => { pos.x += diff; },
        }
        for j in range(i, i + n) {
            app.draw_string(&pos, color, words[j]);
            pos.x += bounds[j] as f32 + app::FONT_SPACE;
        }
        i += n;
        pos.x = origin.x;
        pos.y += app::FONT_HEIGHT;
        line += 1;
    }

    fn num_fitting_words(span: uint, space: uint, lengths: &[uint]) -> (uint, uint) {
        if lengths.len() == 0 { return (0, 0) }
        let mut total = lengths[0];
        for i in range(1, lengths.len()) {
            let new_total = total + space + lengths[i];
            if new_total > span {
                return (i, total);
            }
            total = new_total;
        }
        return (lengths.len(), total);
    }
}

fn outline_print<C: ToRGB>(
    app: &mut App, area: &Aabb2<f32>, color: &C, align: Align, text: &str) {
    print_words(app, &area.add_v(&Vec2::new(-1.0f32, 1.0f32)), &consts::BLACK, align, text);
    print_words(app, area, color, align, text);
}

pub fn main() {
    let mut app = App::new(640, 360, "Shiny!");

    while app.alive {
        app.fill_rect(&RectUtil::new(0.0f32, 0.0f32, 640.0f32, 360.0f32), &consts::MIDNIGHTBLUE);

        let text_zone = Aabb2::new(Point2::new(0.0f32, 200.0f32), Point2::new(240.0f32, 360.0f32));
        outline_print(&mut app, &text_zone,
            &consts::LIGHTSLATEGRAY, Left,
            PANGRAM);

        outline_print(&mut app, &Aabb2::new(Point2::new(260.0f32, 0.0f32), Point2::new(380.0f32, 16.0f32)),
            &consts::CORNFLOWERBLUE, Center,
            "Focus object");

        outline_print(&mut app, &Aabb2::new(Point2::new(560.0f32, 0.0f32), Point2::new(640.0f32, 16.0f32)),
            &consts::LIGHTSLATEGRAY, Right,
            "Area Name");

        for key in app.key_buffer().iter() {
            if key.code == key::ESC {
                return;
            }

            if key.code == key::F12 {
                app.screenshot("/tmp/shot.png");
            }
        }

        app.flush();
    }
}
