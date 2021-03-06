#![allow(clippy::many_single_char_names)]
extern crate serde;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;

#[allow(dead_code)]
pub fn lpca_load_input(filename: &str) -> Result<LpcaInput, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let input = serde_cbor::from_reader(br)?;
    Ok(input)
}

/// a Rust version of lpca (equivalent to the previous commit), just for reference.
/// (the actual performance "problem" is related with lack of fast-math):
///
#[allow(dead_code)]
#[inline]
pub fn lpca(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    lpca1(x, p, r, rc, a)
}

#[allow(dead_code)]
#[inline]
pub fn lpca1(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    let n = x.len();

    for i in 0..=p {
        let mut sum = 0.0f64;
        for k in 0..n - i {
            sum += x[k] * x[k + i];
        }
        r[i] = sum;
    }

    let mut pe: f64 = 0.;
    let r0 = r[0];
    if 0.0f64 == r0 {
        return (1, pe);
    }

    pe = r0;
    a[0] = 1.0f64;
    for k in 1..=p {
        let mut sum = 0.0f64;
        for i in 1..=k {
            sum -= a[k - i] * r[i];
        }
        let akk = sum / pe;
        rc[k] = akk;

        a[k] = akk;
        for i in 1..=k >> 1 {
            let ai = a[i];
            let aj = a[k - i];
            a[i] = ai + akk * aj;
            a[k - i] = aj + akk * ai;
        }

        pe *= 1.0f64 - akk * akk;
        if pe <= 0.0f64 {
            return (2, pe);
        }
    }

    (0, pe)
}

// like lpca1 but with some use of iterators,
// which does seem to improve performance (per cargo bench)
// but making the code not always as readable.
#[allow(dead_code)]
#[inline]
pub fn lpca2(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    let n = x.len();

    for (i, r_i) in r.iter_mut().enumerate() {
        *r_i = x[0..n - i]
            .iter()
            .zip(&x[i..n])
            .map(|(&c, &s)| c * s)
            .sum::<f64>();
    }

    let mut pe: f64 = 0.;
    let r0 = r[0];
    if 0.0f64 == r0 {
        return (1, pe);
    }

    pe = r0;
    a[0] = 1.0f64;
    for k in 1..=p {
        // let mut sum = 0.0f64;
        // for i in 1..=k {
        //   sum -= a[k - i] * r[i];
        // }
        let sum = -a[0..k]
            .iter()
            .rev()
            .zip(&r[1..=k])
            .map(|(&c, &s)| c * s)
            .sum::<f64>();

        let akk = sum / pe;

        rc[k] = akk;
        a[k] = akk;
        for i in 1..=k >> 1 {
            let ai = a[i];
            let aj = a[k - i];
            a[i] = ai + akk * aj;
            a[k - i] = aj + akk * ai;
        }

        pe *= 1.0f64 - akk * akk;
        if pe <= 0.0f64 {
            return (2, pe);
        }
    }

    (0, pe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lpca() {
        let input = lpca_load_input("signal_frame.inputs").unwrap();

        let prediction_order = 36;

        println!(
            "input length={}, prediction_order={}",
            input.x.len(),
            prediction_order
        );

        let mut vector = vec![0f64; prediction_order + 1];
        let mut reflex = vec![0f64; prediction_order + 1];
        let mut pred1 = vec![0f64; prediction_order + 1];

        lpca1(
            &input.x[..],
            prediction_order,
            &mut vector,
            &mut reflex,
            &mut pred1,
        );

        let mut pred2 = vec![0f64; prediction_order + 1];
        lpca2(
            &input.x[..],
            prediction_order,
            &mut vector,
            &mut reflex,
            &mut pred2,
        );

        assert_eq!(pred1, pred2);
    }
}

/// one other equivalent version with some unsafe mechanisms, just for
/// possible reference, but still no actual gain in performance.
///
//#[inline]
//fn lpca_unsafe(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
//    let n = x.len();
//
//    let mut pe: f64 = 0.;
//
//    unsafe {
//        for i in 0..=p {
//            let mut sum = 0.0f64;
//            for k in 0..n - i {
//                //sum += x[k] * x[k + i];
//                let xk = *x.get_unchecked(k);
//                let xki = *x.get_unchecked(k + i);
//                sum += xk * xki;
//            }
//            *r.get_unchecked_mut(i) = sum;
//            //r[i] = sum;
//        }
//        let r0 = *r.get_unchecked(0);
//        //let r0 = r[0];
//        if 0.0f64 == r0 {
//            return (1, pe);
//        }
//
//        pe = r0;
//        *a.get_unchecked_mut(0) = 1.0f64;
//        //a[0] = 1.0f64;
//        for k in 1..=p {
//            let mut sum = 0.0f64;
//            for i in 1..=k {
//                //sum -= a[k - i] * r[i];
//                let aki = *a.get_unchecked(k - i);
//                let ri = *r.get_unchecked(i);
//                sum -= aki * ri;
//            }
//            let akk = sum / pe;
//            *rc.get_unchecked_mut(k) = akk;
//            //rc[k] = akk;
//
//            a[k] = akk;
//            for i in 1..=k >> 1 {
//                //let ai = a[i];
//                //let aj = a[k - i];
//                //a[i] = ai + akk * aj;
//                //a[k - i] = aj + akk * ai;
//                let ai = *a.get_unchecked(i);
//                let aj = *a.get_unchecked(k - i);
//                *a.get_unchecked_mut(i) = ai + akk * aj;
//                *a.get_unchecked_mut(k - i) = aj + akk * ai;
//            }
//
//            pe *= 1.0f64 - akk * akk;
//            if pe <= 0.0f64 {
//                return (2, pe);
//            }
//        }
//    }
//
//    (0, pe)
//}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LpcaInput {
    pub x: Vec<f64>,
    pub p: usize,
}

impl LpcaInput {
    fn save(&mut self, filename: &str) {
        let f = File::create(filename).unwrap();
        let bw = BufWriter::new(f);
        serde_cbor::to_writer(bw, &self).unwrap();
    }
}

pub fn lpca_save_input(x: &[f64], p: usize, filename: &str) {
    let mut input = LpcaInput { x: x.to_vec(), p };
    input.save(filename)
}
