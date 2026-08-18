use std::hint::black_box;

use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main};
use plant_evolution_lib::utils::formula::*;

extern crate plant_evolution_lib;

const SIMPLE_N: usize = 1_000_000;

/*
    Simple: (a + b) / c * d
*/

fn get_simple_tree() -> Vec<FormulaNode<ArrayIdx<4>>> {
    let b: FormulaBuilder<ArrayIdx<4>> = FormulaBuilder::new();
    b.binary_operator(BinaryOp::Mul, 
        b.binary_operator(BinaryOp::Div, 
            b.binary_operator(BinaryOp::Add, 
                b.parameter(0), 
            b.parameter(1)), 
        b.parameter(2)), 
    b.parameter(3)).build()
}

fn do_test<'a, M: criterion::measurement::Measurement, F: Formula<[f32; 4]>>(group: &mut BenchmarkGroup<'a, M>, input: &[[f32; 4]; SIMPLE_N], name: &str, f: F) {
    group.bench_function(name, |b| {
        b.iter(|| {
            (0..SIMPLE_N).for_each(|i| {
                black_box(f.calculate(&input[i]));
            });
        });
    });
}

fn benchmark_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("formula-benches-simple");

    let input: [[f32; 4]; SIMPLE_N] = std::array::from_fn(|i| {
        [
            1000. - i as f32 * 4.,
            i as f32 / 2.,
            ((i + 1) as f32 * 153.).sqrt(),
            44. + (i as f32 * 0.12),
        ]
    });

    let formula_tree = get_simple_tree();
    let formula_str = "(a + b) / c * d".to_owned();

    let tree_formula = TreeFormula::new(formula_tree.clone());
    let array_formula = TreeArrayFormula::new(formula_tree);
    let meval_formula = MEvalFormula::<ArrayIdx<4>>::new(formula_str.clone()).unwrap();
    let tabulon_formula = TabulonFormula::<ArrayIdx<4>>::new(formula_str.clone()).unwrap();
    let mathexpr_formula = MathexprFormula::<ArrayIdx<4>>::new(formula_str.clone()).unwrap();

    group.bench_function("native", |b| {
        b.iter(|| {
            (0..SIMPLE_N).for_each(|i| {
                black_box((input[i][0] + input[i][1]) / input[i][2] * input[i][3]);
            });
        });
    });

    do_test(&mut group, &input, "tree_formula", tree_formula);
    do_test(&mut group, &input, "array_formula", array_formula);
    do_test(&mut group, &input, "meval_formula", meval_formula);
    do_test(&mut group, &input, "tabulon_formula", tabulon_formula);
    do_test(&mut group, &input, "mathexpr_formula", mathexpr_formula);

    group.finish();
}

criterion_group!(benches, benchmark_simple);
criterion_main!(benches);
