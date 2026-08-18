use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use plant_evolution_lib::utils::formula;

extern crate plant_evolution_lib;

const SIMPLE_N: usize = 1_000_000;
const COMPLEX_N: usize = 100_000;

/*
    Simple: (a + b) / c * d
    Complex: (a.powi(4) / b.sqrt() + (c^-1).ln() - d.powi(2)).sqrt()
*/

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
enum SimplePId {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
}

impl formula::ParameterId for SimplePId {
    fn get_name(&self) -> String {
        String::default()
    }
}

impl formula::Parameters<SimplePId> for [f32; 4] {
    fn get_value(&self, id: &SimplePId) -> f32 {
        self[*id as usize]
    }
}

fn simple_native(input: &[[f32; 4]]) -> Vec<f32> {
    input
        .iter()
        .map(|[a, b, c, d]| black_box((a + b) / c * d))
        .collect()
}

fn simple_formula(input: &[[f32; 4]]) -> Vec<f32> {
    use plant_evolution_lib::utils::formula::*;

    let formula = TreeFormula::new(vec![
        FormulaNode::Operation(OpNode::Binary(BinaryOp::Mul, 1, 2)),
        FormulaNode::Operation(OpNode::Binary(BinaryOp::Div, 3, 4)),
        FormulaNode::Parameter(SimplePId::D),
        FormulaNode::Operation(OpNode::Binary(BinaryOp::Add, 5, 6)),
        FormulaNode::Parameter(SimplePId::C),
        FormulaNode::Parameter(SimplePId::A),
        FormulaNode::Parameter(SimplePId::B),
    ]);
    input
        .iter()
        .map(|inp| black_box(formula.calculate(inp)))
        .collect()
}

fn simple_meval(input: &[[f32; 4]]) -> Vec<f32> {
    use meval::Expr;

    let expr: Expr = "(a + b) / c * d".parse().unwrap();
    let formula = expr.bind4("a", "b", "c", "d").unwrap();
    input
        .iter()
        .map(|inp| {
            black_box(formula(
                inp[0] as f64,
                inp[1] as f64,
                inp[2] as f64,
                inp[3] as f64,
            )) as f32
        })
        .collect()
}

fn simple_tabulon(input: &[[f32; 4]]) -> Vec<f32> {
    use tabulon::Tabula;

    let mut engine = Tabula::new();
    let expr = engine.compile("(a + b) / c * d").unwrap();
    assert_eq!(expr.vars(), &["a", "b", "c", "d"]);

    let mut data = vec![0.; 4];
    input
        .iter()
        .map(|inp| {
            data[0] = inp[0] as f64;
            data[1] = inp[1] as f64;
            data[2] = inp[2] as f64;
            data[3] = inp[3] as f64;
            black_box(expr.eval(&data)).unwrap() as f32
        })
        .collect()
}

fn simple_evalexprjit(input: &[[f32; 4]]) -> Vec<f32> {
    use evalexpr_jit::equation::Equation;

    let eq = Equation::new("(a + b) / c * d".to_string()).unwrap();
    let mut data = vec![0.; 4];
    input
        .iter()
        .map(|inp| {
            data[0] = inp[0] as f64;
            data[1] = inp[1] as f64;
            data[2] = inp[2] as f64;
            data[3] = inp[3] as f64;
            black_box(eq.eval(&data)).unwrap() as f32
        })
        .collect()
}

fn complex_native(input: &[[f32; 4]]) -> Vec<f32> {
    input
        .iter()
        .map(|[a, b, c, d]| black_box((a.powi(4) / b.sqrt() + c.powi(-1).ln() - d.powi(2)).sqrt()))
        .collect()
}

fn complex_formula(input: &[[f32; 4]]) -> Vec<f32> {
    use plant_evolution_lib::utils::formula::*;

    let formula = TreeFormula::new(vec![
        /*0*/
        FormulaNode::Operation(OpNode::Unary(UnaryOp::Sqrt, 1)), // (a.powi(4) / b.sqrt() + (c^-1).ln() - d.powi(2)).SQRT()
        /*1*/
        FormulaNode::Operation(OpNode::Binary(BinaryOp::Sub, 2, 3)), // a.powi(4) / b.sqrt() + (c^-1).ln() SUB d.powi(2)
        /*2*/
        FormulaNode::Operation(OpNode::Binary(BinaryOp::Add, 4, 5)), // a.powi(4) / b.sqrt() ADD (c^-1).ln()
        /*3*/ FormulaNode::Operation(OpNode::Unary(UnaryOp::Sqr, 6)), // d.POWI(2)
        /*4*/
        FormulaNode::Operation(OpNode::Binary(BinaryOp::Div, 7, 8)), // a.powi(4) DIV b.sqrt()
        /*5*/ FormulaNode::Operation(OpNode::Unary(UnaryOp::Ln, 9)), // (c^-1).LN()
        /*6*/ FormulaNode::Parameter(SimplePId::D), // D
        /*7*/
        FormulaNode::Operation(OpNode::Binary(BinaryOp::Powi, 10, 13)), // a.POWI(4)
        /*8*/ FormulaNode::Operation(OpNode::Unary(UnaryOp::Sqrt, 11)), // b.SQRT()
        /*9*/ FormulaNode::Operation(OpNode::Unary(UnaryOp::Inv, 12)), // c POW -1
        /*10*/ FormulaNode::Parameter(SimplePId::A), // A
        /*11*/ FormulaNode::Parameter(SimplePId::B), // B
        /*12*/ FormulaNode::Parameter(SimplePId::C), // C
        /*13*/ FormulaNode::Value(4.),
    ]);
    input
        .iter()
        .map(|inp| black_box(formula.calculate(inp)))
        .collect()
}

fn complex_meval(input: &[[f32; 4]]) -> Vec<f32> {
    use meval::Expr;

    let expr: Expr = "sqrt(a^4 / sqrt(b) + ln(1/c) - d^2)".parse().unwrap();
    let formula = expr.bind4("a", "b", "c", "d").unwrap();
    input
        .iter()
        .map(|inp| {
            black_box(formula(
                inp[0] as f64,
                inp[1] as f64,
                inp[2] as f64,
                inp[3] as f64,
            )) as f32
        })
        .collect()
}

// (a.powi(4) / b.sqrt() + (c^-1).ln() - d.powi(2)).sqrt()
fn complex_evalexprjit(input: &[[f32; 4]]) -> Vec<f32> {
    use evalexpr_jit::equation::Equation;

    let eq = Equation::new("sqrt(a^4 / sqrt(b) + ln(1/c) - d^2)".to_string()).unwrap();
    let mut data = vec![0.; 4];
    input
        .iter()
        .map(|inp| {
            data[0] = inp[0] as f64;
            data[1] = inp[1] as f64;
            data[2] = inp[2] as f64;
            data[3] = inp[3] as f64;
            black_box(eq.eval(&data)).unwrap() as f32
        })
        .collect()
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

    {
        let test_input = &input[..1000];
        let result = simple_native(test_input);
        const EPS: f32 = 1e-3;
        let compare = |v1: &[f32], v2: &[f32]| {
            assert_eq!(v1.len(), v2.len());
            v1.iter().zip(v2.iter()).for_each(|(v1, v2)| {
                if (v1 - v2).abs() > EPS {
                    panic!("{v1} {v2}");
                }
            });
        };
        compare(&result, &simple_formula(test_input));
        //compare(&result, &simple_formula::<ArrayRuntime>(test_input));
        compare(&result, &simple_meval(test_input));
        compare(&result, &simple_tabulon(test_input));
        compare(&result, &simple_evalexprjit(test_input));
    }

    let mut test = |name, f: &dyn Fn(&[[f32; 4]]) -> Vec<f32>| {
        group.bench_function(name, |b| {
            b.iter(|| f(&input));
        });
    };

    test("simple-native", &simple_native);
    test("simple-formula-tree", &simple_formula);
    //test("simple-formula-array", &simple_formula::<ArrayRuntime>);
    test("simple-meval", &simple_meval);
    test("simple-tabulon", &simple_tabulon);
    test("simple-evalexprjit", &simple_evalexprjit);

    group.finish();
}

fn benchmark_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("formula-benches-complex");

    let input_complex: [[f32; 4]; COMPLEX_N] = std::array::from_fn(|i| {
        [
            1000. - i as f32 * 4.,
            i as f32 / 2.,
            ((i + 1) as f32 * 153.).sqrt(),
            44. + (i as f32 * 0.12),
        ]
    });

    {
        let test_input = &input_complex[..1000];
        let result = complex_native(test_input);
        const EPS: f32 = 1.;
        let compare = |v1: &[f32], v2: &[f32]| {
            assert_eq!(v1.len(), v2.len());
            v1.iter().zip(v2.iter()).for_each(|(v1, v2)| {
                if (v1 - v2).abs() > EPS {
                    panic!("{v1} {v2}");
                }
            });
        };
        compare(&result, &complex_formula(test_input));
        //compare(&result, &complex_formula::<ArrayRuntime>(test_input));
        compare(&result, &complex_meval(test_input));
        compare(&result, &complex_evalexprjit(test_input));
    }

    let mut test = |name, f: &dyn Fn(&[[f32; 4]]) -> Vec<f32>| {
        group.bench_function(name, |b| {
            b.iter(|| f(&input_complex));
        });
    };

    test("complex-native", &complex_native);
    test("complex-formula-tree", &complex_formula);
    //test("complex-formula-array", &complex_formula::<ArrayRuntime>);
    test("complex-meval", &complex_meval);
    test("complex-evalexprjit", &complex_evalexprjit);

    group.finish();
}

criterion_group!(benches, benchmark_simple, benchmark_complex);
criterion_main!(benches);
