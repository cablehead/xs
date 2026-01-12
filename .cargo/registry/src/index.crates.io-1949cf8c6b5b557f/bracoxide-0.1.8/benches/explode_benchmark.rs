use bracoxide::bracoxidize;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn explode_benchmark(c: &mut Criterion) {
    let content = black_box("mkdir -p /home/X/{Videos/{Movies/{Action,Adventure,Horror},Series},Documents/{pdf,epub},Temp{3..15}}");
    c.bench_function("explode benchmark", |b| {
        b.iter(|| bracoxidize(content));
    });
}

criterion_group!(benches, explode_benchmark);
criterion_main!(benches);
