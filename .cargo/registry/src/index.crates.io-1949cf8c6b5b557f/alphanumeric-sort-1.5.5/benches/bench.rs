use std::path::Path;

use bencher::{benchmark_group, benchmark_main, Bencher};

#[derive(Debug)]
struct P(&'static Path);

impl From<&'static str> for P {
    #[inline]
    fn from(s: &'static str) -> Self {
        P(Path::new(s))
    }
}

fn sort_slice_by_path_key(bencher: &mut Bencher) {
    bencher.iter(|| {
        let mut array = [
            P::from("第10-15-2章"),
            P::from("第1-2章"),
            P::from("第2-4章"),
            P::from("第2-33章"),
            P::from("第1章"),
            P::from("第1-4章"),
            P::from("第2-3章"),
            P::from("第1-11章"),
            P::from("第10-1章"),
            P::from("第3-1章"),
            P::from("第2-10章"),
            P::from("第2-2章"),
            P::from("第1-3章"),
            P::from("第10-15章"),
            P::from("第10-2章"),
            P::from("第10-15-1章"),
            P::from("第2-1章"),
            P::from("第2-12章"),
            P::from("第1-10章"),
            P::from("第3-10章"),
        ];

        alphanumeric_sort::sort_slice_by_path_key(&mut array, |e| e.0);

        array
    });
}

fn sort_path_slice(bencher: &mut Bencher) {
    bencher.iter(|| {
        let mut array = [
            Path::new("第10-15-2章"),
            Path::new("第1-2章"),
            Path::new("第2-4章"),
            Path::new("第2-33章"),
            Path::new("第1章"),
            Path::new("第1-4章"),
            Path::new("第2-3章"),
            Path::new("第1-11章"),
            Path::new("第10-1章"),
            Path::new("第3-1章"),
            Path::new("第2-10章"),
            Path::new("第2-2章"),
            Path::new("第1-3章"),
            Path::new("第10-15章"),
            Path::new("第10-2章"),
            Path::new("第10-15-1章"),
            Path::new("第2-1章"),
            Path::new("第2-12章"),
            Path::new("第1-10章"),
            Path::new("第3-10章"),
        ];

        alphanumeric_sort::sort_path_slice(&mut array);

        array
    });
}

fn compare_path(bencher: &mut Bencher) {
    bencher.iter(|| {
        let mut array = [
            Path::new("第10-15-2章"),
            Path::new("第1-2章"),
            Path::new("第2-4章"),
            Path::new("第2-33章"),
            Path::new("第1章"),
            Path::new("第1-4章"),
            Path::new("第2-3章"),
            Path::new("第1-11章"),
            Path::new("第10-1章"),
            Path::new("第3-1章"),
            Path::new("第2-10章"),
            Path::new("第2-2章"),
            Path::new("第1-3章"),
            Path::new("第10-15章"),
            Path::new("第10-2章"),
            Path::new("第10-15-1章"),
            Path::new("第2-1章"),
            Path::new("第2-12章"),
            Path::new("第1-10章"),
            Path::new("第3-10章"),
        ];

        array.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));

        array
    });
}

benchmark_group!(sort_path_compare_path, sort_slice_by_path_key, sort_path_slice, compare_path);
benchmark_main!(sort_path_compare_path);
