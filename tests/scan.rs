use ndarray::Array2;

use zhang_hilbert::HilbertScan32;

#[test]
fn scan32_patterns() {
    for w in 2..32 {
        for h in 2..32 {
            println!("=== {:?} ===", [w, h]);
            let scan = HilbertScan32::new([w, h]);
            let mut map: Array2<usize> = Array2::zeros([h as usize, w as usize]);

            for (i, x) in scan.enumerate() {
                println!("{:?}", x);
                if map[[x[1] as usize, x[0] as usize]] != 0 {
                    println!("{:?} has been already visited. \nMap: {:#?}", x, &map);
                }
                map[[x[1] as usize, x[0] as usize]] = i + 1;
            }

            for ((y, x), value) in map.indexed_iter() {
                if *value == 0 {
                    panic!("{:?} was never visited. \nMap: {:#?}", [x, y], &map);
                }
            }
        }
    }
}
