#![feature(test)]
#![allow(non_snake_case)]
extern crate test;

use zhang_hilbert::HilbertScan32;

fn scan32_run(size: [u32; 2], b: &mut test::Bencher) {
    b.iter(|| -> u32 { HilbertScan32::new(size).map(|[x, y]| x + y).sum() })
}

#[bench]
fn scan32____4____4(b: &mut test::Bencher) {
    scan32_run([4, 4], b);
}

#[bench]
fn scan32___16___16(b: &mut test::Bencher) {
    scan32_run([16, 16], b);
}

#[bench]
fn scan32__256__256(b: &mut test::Bencher) {
    scan32_run([256, 256], b);
}

#[bench]
fn scan32__114__514(b: &mut test::Bencher) {
    scan32_run([114, 514], b);
}
