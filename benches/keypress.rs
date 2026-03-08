use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use igrepper::igrepper::core::Core;
use igrepper::igrepper::state::{SearchLine, State};
use rand::{Rng, SeedableRng};

fn generate_source_lines(n: usize) -> Vec<String> {
    let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
    (0..n)
        .map(|_| {
            let len = rng.gen_range(50..200);
            (0..len)
                .map(|_| rng.gen_range(b'a'..=b'z') as char)
                .collect()
        })
        .collect()
}

const SIZES: [usize; 2] = [100_000, 1000_000];
const SMALL_SIZES: [usize; 2] = [1000, 10_000];

fn bench_type_character(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_character");
    for &size in &SIZES {
        let source = generate_source_lines(size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}")),
            &size,
            |b, _| {
                b.iter(|| {
                    let mut core = Core::new();
                    let state = State::new(
                        source.clone(),
                        vec![SearchLine::new(String::from(""), 0, false, false)],
                        0,
                        0,
                        64,
                        230,
                    );
                    let state = state.push_search_char('.');
                    std::hint::black_box(core.get_render_state(&state))
                })
            },
        );
    }
    group.finish();
}

fn bench_type_and_backspace(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_and_backspace");
    for &size in &SIZES {
        let source = generate_source_lines(size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}")),
            &size,
            |b, _| {
                b.iter(|| {
                    let mut core = Core::new();
                    let state = State::new(
                        source.clone(),
                        vec![SearchLine::new(String::from(""), 0, false, false)],
                        0,
                        0,
                        64,
                        230,
                    );
                    let state = state.push_search_char('.');
                    std::hint::black_box(core.get_render_state(&state));
                    let state = state.pop_search_char();
                    std::hint::black_box(core.get_render_state(&state))
                })
            },
        );
    }
    group.finish();
}

fn bench_type_character_and_accept(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_character_and_accept");
    for &size in &SMALL_SIZES {
        let source = generate_source_lines(size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}")),
            &size,
            |b, _| {
                b.iter(|| {
                    let mut core = Core::new();
                    let state = State::new(
                        source.clone(),
                        vec![SearchLine::new(String::from(""), 0, false, false)],
                        0,
                        0,
                        64,
                        230,
                    );
                    let state = state.push_search_char('.');
                    let state = std::hint::black_box(state).accept_partial_match();
                    std::hint::black_box(core.get_render_state(&state))
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_type_character,
    bench_type_and_backspace,
    bench_type_character_and_accept,
);
criterion_main!(benches);
