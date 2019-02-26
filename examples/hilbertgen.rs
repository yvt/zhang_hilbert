use ndarray::{s, Array2};
use zhang_hilbert::{ArbHilbertScan32, HilbertScan32};

fn main() {
    use clap::{App, Arg};
    // Use `clap` to parse command-line arguments
    let matches = App::new("hilbertgen")
        .about("Generates a pseudo-Hilbert curve")
        .arg(
            Arg::with_name("WIDTH")
                .help("Width of the generated scan")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("HEIGHT")
                .help("Height of the generated scan")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .help("Set the output format")
                .takes_value(true)
                .possible_values(&["ascii", "svg", "json", "csv", "tsv"])
                .default_value("ascii"),
        )
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

    let size_w: u32 = matches
        .value_of("WIDTH")
        .and_then(|x| x.parse().ok())
        .expect("Invalid width");
    let size_h: u32 = matches
        .value_of("HEIGHT")
        .and_then(|x| x.parse().ok())
        .expect("Invalid height");

    let algo = matches.value_of("algorithm").unwrap();
    let scan: Box<dyn Iterator<Item = [u32; 2]>> = if algo == "zhang" {
        Box::new(HilbertScan32::new([size_w, size_h]))
    } else if algo == "zhang-arb" {
        Box::new(ArbHilbertScan32::new([size_w, size_h]))
    } else {
        unreachable!()
    };

    let format = matches.value_of("format").unwrap();

    if format == "ascii" {
        // Warning: The coordinate space here is upside down - +Y is down, -Y is up
        let mut grid: Array2<char> =
            Array2::from_shape_fn((size_h as usize, size_w as usize * 2 - 1), |_| ' ');
        let mut p: Option<[i32; 2]> = None;
        let mut last_dir: Option<Dir> = None;
        for [x, y] in scan {
            let [x, y] = [x as i32 * 2, (size_h - 1 - y) as i32];
            if let Some([mut ox, mut oy]) = p {
                if ox != x {
                    assert!(oy == y);

                    let dir = (x - ox).signum();

                    grid[[oy as usize, ox as usize]] = match (last_dir, dir) {
                        (None, _) | (Some(Dir::PosX), _) | (Some(Dir::NegX), _) => '-',
                        (Some(Dir::NegY), _) => ',',
                        (Some(Dir::PosY), _) => '\'',
                    };
                    last_dir = match dir {
                        1 => Some(Dir::PosX),
                        -1 => Some(Dir::NegX),
                        _ => unreachable!(),
                    };

                    while ox != x {
                        ox += (x - ox).signum();
                        grid[[oy as usize, ox as usize]] = '-';
                    }
                } else if oy != y {
                    let dir = (y - oy).signum();

                    grid[[oy as usize, ox as usize]] = match (last_dir, dir) {
                        (None, _) | (Some(Dir::PosY), _) | (Some(Dir::NegY), _) => '|',
                        (_, 1) => ',',
                        (_, -1) => '\'',
                        _ => unreachable!(),
                    };
                    last_dir = match dir {
                        1 => Some(Dir::PosY),
                        -1 => Some(Dir::NegY),
                        _ => unreachable!(),
                    };

                    while oy != y {
                        oy += (y - oy).signum();
                        grid[[oy as usize, ox as usize]] = '|';
                    }
                }
            }
            p = Some([x, y]);
        }
        for y in 0..size_h as usize {
            let slice = grid.slice(s![y, ..]);
            let s: String = slice.iter().cloned().collect();
            println!("{}", s);
        }
    } else if format == "json" {
        println!("[");
        let mut scan = scan.peekable();
        while let Some([x, y]) = scan.next() {
            if scan.peek().is_some() {
                println!("  [{}, {}],", x, y);
            } else {
                println!("  [{}, {}]", x, y);
            }
        }
        println!("]");
    } else if format == "csv" {
        for [x, y] in scan {
            println!("{}, {}", x, y);
        }
    } else if format == "tsv" {
        for [x, y] in scan {
            println!("{}\t{}", x, y);
        }
    } else if format == "svg" {
        const SCALE: u32 = 10;
        println!(r#"<?xml version="1.0" encoding="utf-8"?>"#);
        println!(
            r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg"
            xmlns:xlink="http://www.w3.org/1999/xlink" x="0px" y="0px"
            viewBox="0 0 {} {}">"#,
            (size_w + 1) * SCALE,
            (size_h + 1) * SCALE,
        );
        print!(r#"<path d=""#);
        for (i, [x, y]) in scan.enumerate() {
            let cmd = if i == 0 { 'M' } else { 'L' };
            print!(
                "{}{},{}",
                cmd,
                (x + 1) * SCALE,
                (size_h - 1 - y + 1) * SCALE
            );
        }
        println!(r#"" fill="none" stroke="black"/>"#);
        println!(r#"</svg>"#);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Dir {
    PosX,
    NegX,
    PosY,
    NegY,
}
