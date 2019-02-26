use ndarray::Array2;

use zhang_hilbert::{ArbHilbertScan32, HilbertScan32};

fn validate_curve(scan: impl Iterator<Item = [u32; 2]>, [w, h]: [u32; 2]) {
    let mut map: Array2<usize> = Array2::zeros([h as usize, w as usize]);

    let mut last: Option<[u32; 2]> = None;

    for (i, x) in scan.enumerate() {
        println!("{:?}", x);
        if map[[x[1] as usize, x[0] as usize]] != 0 {
            println!("{:?} has been already visited. \nMap: {:#?}", x, &map);
        }
        map[[x[1] as usize, x[0] as usize]] = i + 1;
        if let Some(last) = last {
            assert!(
                (last[0] != x[0]) != (last[1] != x[1]),
                "Invalid move: {:?} → {:?}. \nMap: {:#?}",
                last,
                x,
                &map
            );
        }
        last = Some(x);
    }

    for ((y, x), value) in map.indexed_iter() {
        if *value == 0 {
            panic!("{:?} was never visited. \nMap: {:#?}", [x, y], &map);
        }
    }
}

#[test]
fn normal_scan32_patterns() {
    for w in 0..32 {
        for h in 0..32 {
            println!("=== {:?} ===", [w, h]);
            let scan = HilbertScan32::new([w, h]);
            validate_curve(scan, [w, h]);
        }
    }
}

#[test]
fn arb_scan32_patterns() {
    for w in 0..32 {
        for h in 0..32 {
            println!("=== {:?} ===", [w, h]);
            let scan = ArbHilbertScan32::new([w, h]);
            validate_curve(scan, [w, h]);
        }
    }
}
