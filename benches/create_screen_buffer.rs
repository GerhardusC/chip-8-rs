use std::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};

fn create_buffer_string(buf: &[u8], screen_width: usize) -> String {
    buf.chunks(screen_width).map(|chars| {
        chars.iter().map(|c| {
            if *c > 0 {'█'} else {' '}
        }).collect()
    }).collect::<Vec<String>>().join("\n")
}

fn create_one_alloc_buffer_string(buf: &[u8], screen_width: usize) -> String {
    let mut s = String::with_capacity(buf.len());
    for (i, b) in buf.iter().enumerate() {
        if (i % screen_width) == 0 {
            s.push('\n');
        }
        if *b > 0 {
            s.push('█');
        } else {
            s.push(' ');
        };
    }
    s.push('\n');
    s
}

fn create_buffer_string_iterator(c: &mut Criterion) {
    let vals: [u8; 0xFFFF] = [0; 0xFFFF];
    c.bench_function("Iterator Approach", |b| b.iter(|| create_buffer_string(&vals, black_box(256))));
}

fn create_buffer_string_push(c: &mut Criterion) {
    let vals: [u8; 0xFFFF] = [0; 0xFFFF];
    c.bench_function("Single Allocation Approach", |b| b.iter(|| create_one_alloc_buffer_string(&vals, black_box(256))));
}

criterion_group!(benches, create_buffer_string_iterator, create_buffer_string_push);
criterion_main!(benches);
