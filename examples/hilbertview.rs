use sdl2::{
    event::{Event, WindowEvent},
    keyboard::Keycode,
    rect::Point,
    render::{Canvas, RenderTarget},
};
use std::cmp::max;
use zhang_hilbert::{ArbHilbertScan32, HilbertScan32};

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("hilbertview")
        .about("Displays a pseudo-Hilbert curve in a resizable window")
        .arg(
            Arg::with_name("algorithm")
                .short("a")
                .long("algorithm")
                .help("Set the algorithm")
                .takes_value(true)
                .possible_values(&["zhang", "zhang-arb"])
                .default_value("zhang-arb"),
        )
        .get_matches();

    let algo = matches.value_of("algorithm").unwrap();
    let points_generator = if algo == "zhang" {
        make_points_generator(|size| HilbertScan32::new(size))
    } else if algo == "zhang-arb" {
        make_points_generator(|size| ArbHilbertScan32::new(size))
    } else {
        unreachable!()
    };

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("hilbertview", 800, 500)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    render(&mut canvas, &points_generator);

    'running: loop {
        for event in event_pump.wait_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::Window {
                    win_event: WindowEvent::Resized { .. },
                    ..
                }
                | Event::Window {
                    win_event: WindowEvent::SizeChanged { .. },
                    ..
                } => {
                    render(&mut canvas, &points_generator);
                }
                _ => {}
            }
        }
    }
}

const SCALE: u32 = 10;

fn make_points_generator<I: Iterator<Item = [u32; 2]>>(
    f: impl Fn([u32; 2]) -> I + 'static,
) -> Box<dyn Fn([u32; 2]) -> Vec<Point>> {
    Box::new(move |size| {
        let mut points: Vec<Point> = Vec::with_capacity((size[0] * size[1]) as usize);
        points.extend(f(size).map(|[x, y]| -> Point {
            (((x + 1) * SCALE) as i32, ((y + 1) * SCALE) as i32).into()
        }));
        points
    })
}

fn render<T: RenderTarget>(
    canvas: &mut Canvas<T>,
    points_generator: &Box<dyn Fn([u32; 2]) -> Vec<Point>>,
) {
    let (canvas_w, canvas_h) = canvas.output_size().unwrap();

    let size_w = max(canvas_w, SCALE) / SCALE - 1;
    let size_h = max(canvas_h, SCALE) / SCALE - 1;

    canvas.set_draw_color((0, 0, 0));
    canvas.clear();

    let points = points_generator([size_w, size_h]);
    canvas.set_draw_color((64, 255, 64));
    canvas.draw_lines(&points[..]).unwrap();

    canvas.present();
}
