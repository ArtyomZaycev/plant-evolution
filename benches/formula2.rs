use std::hint::black_box;

use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main};
use plant_evolution_lib::utils::formula::*;
use rand::{RngExt, SeedableRng, rngs::SmallRng};

extern crate plant_evolution_lib;

const SIMPLE_N: usize = 1_000_000;

/*
    Simple: (a + b) / c * d
*/

fn get_simple_tree() -> Vec<FormulaNode<ArrayIdx<4>>> {
    let b: FormulaBuilder<ArrayIdx<4>> = FormulaBuilder::new();
    b.binary_operator(
        BinaryOp::Mul,
        b.binary_operator(
            BinaryOp::Div,
            b.binary_operator(BinaryOp::Add, b.parameter(0), b.parameter(1)),
            b.parameter(2),
        ),
        b.parameter(3),
    )
    .build()
}

const RNG_SEED: u64 = 123;
fn simple_input_fn(rng: &mut SmallRng, _: usize) -> [f32; 4] {
    rng.random()
}

fn do_test<'a, M: criterion::measurement::Measurement, F: Formula<[f32; 4]>>(
    group: &mut BenchmarkGroup<'a, M>,
    name: &str,
    f: F,
) {
    group.bench_function(name, |b| {
        b.iter(|| {
            let mut rng = SmallRng::seed_from_u64(RNG_SEED);
            (0..SIMPLE_N).for_each(|i| {
                black_box(f.calculate(&simple_input_fn(&mut rng, i)));
            });
        });
    });
}

fn benchmark_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("formula-benches-simple");

    let formula_tree = get_simple_tree();
    let formula_str = "(a + b) / c * d".to_owned();

    let tree_formula = TreeFormula::new(formula_tree.clone());
    let array_formula = TreeArrayFormula::new(formula_tree);
    let meval_formula = MEvalFormula::<ArrayIdx<4>>::new(formula_str.clone()).unwrap();
    let tabulon_formula = TabulonFormula::<ArrayIdx<4>>::new(formula_str.clone()).unwrap();
    let mathexpr_formula = MathexprFormula::<ArrayIdx<4>>::new(formula_str.clone()).unwrap();
    let xprs_formula = XprsFormula::<ArrayIdx<4>>::new(formula_str.clone()).unwrap();

    group.bench_function("native", |b| {
        b.iter(|| {
            let mut rng = SmallRng::seed_from_u64(RNG_SEED);
            (0..SIMPLE_N).for_each(|i| {
                let input = simple_input_fn(&mut rng, i);
                black_box((input[0] + input[1]) / input[2] * input[3]);
            });
        });
    });

    do_test(&mut group, "tree_formula", tree_formula);
    do_test(&mut group, "array_formula", array_formula);
    do_test(&mut group, "meval_formula", meval_formula);
    do_test(&mut group, "tabulon_formula", tabulon_formula);
    do_test(&mut group, "mathexpr_formula", mathexpr_formula);
    do_test(&mut group, "xprs_formula", xprs_formula);

    group.finish();
}

criterion_group!(benches, benchmark_simple);
criterion_main!(benches);
