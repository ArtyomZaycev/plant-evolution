use super::parameters::*;

fn traverse_inner<PId: ParameterId>(nodes: &mut Vec<FormulaNode<PId>>, f: &mut Vec<bool>, idx: usize) {
    f[idx] = true;
    match nodes[idx] {
        FormulaNode::Value(_) => {}
        FormulaNode::Parameter(_) => {}
        FormulaNode::Operation(op_node) => match op_node {
            OpNode::Unary(_, idx1) => {
                traverse_inner(nodes, f, idx1);
            }
            OpNode::Binary(_, idx1, idx2) => {
                traverse_inner(nodes, f, idx1);
                traverse_inner(nodes, f, idx2);
            }
        },
    }
}

pub(super) fn compact<PId: ParameterId>(nodes: &mut Vec<FormulaNode<PId>>) {
    let mut f = vec![false; nodes.len()];
    traverse_inner(nodes, &mut f, 0);
    let mut new_idx = vec![0; nodes.len()];
    f.iter().enumerate().fold(0, |cnt, (i, v)| {
        new_idx[i] = i - cnt;
        if *v {
            cnt
        } else {
            nodes.remove(i - cnt);
            cnt + 1
        }
    });
    nodes.iter_mut().for_each(|node| {
        if let FormulaNode::Operation(op_node) = node {
            match op_node {
                OpNode::Unary(_, idx1) => {
                    *idx1 = new_idx[*idx1];
                }
                OpNode::Binary(_, idx1, idx2) => {
                    *idx1 = new_idx[*idx1];
                    *idx2 = new_idx[*idx2];
                }
            }
        }
    });
}

fn get_subformula<PId: ParameterId>(nodes: &Vec<FormulaNode<PId>>, idx: usize) -> String {
    match &nodes[idx] {
        FormulaNode::Value(value) => format!("{:.2}", value),
        FormulaNode::Parameter(id) => id.get_name().clone(),
        FormulaNode::Operation(op_node) => match op_node {
            OpNode::Unary(unary_op, idx1) => match unary_op {
                UnaryOp::Sqr => format!("{}^2", get_subformula(nodes, *idx1)),
                UnaryOp::Sqrt => format!("sqrt({})", get_subformula(nodes, *idx1)),
                UnaryOp::Pow(n) => format!("({})^{}", get_subformula(nodes, *idx1), n),
                UnaryOp::Powi(n) => format!("({})^{}", get_subformula(nodes, *idx1), n),
                UnaryOp::Ln => format!("ln({})", get_subformula(nodes, *idx1)),
                UnaryOp::Inv => format!("{}^-1", get_subformula(nodes, *idx1)),
                UnaryOp::Minus => format!("-{}", get_subformula(nodes, *idx1)),
            },
            OpNode::Binary(binary_op, idx1, idx2) => match binary_op {
                BinaryOp::Add => format!(
                    "({} + {})",
                    get_subformula(nodes, *idx1),
                    get_subformula(nodes, *idx2)
                ),
                BinaryOp::Sub => format!(
                    "({} - {})",
                    get_subformula(nodes, *idx1),
                    get_subformula(nodes, *idx2)
                ),
                BinaryOp::Mul => format!(
                    "({} * {})",
                    get_subformula(nodes, *idx1),
                    get_subformula(nodes, *idx2)
                ),
                BinaryOp::Div => format!(
                    "({} / {})",
                    get_subformula(nodes, *idx1),
                    get_subformula(nodes, *idx2)
                ),
            },
        },
    }
}

pub fn get_formula<PId: ParameterId>(nodes: &Vec<FormulaNode<PId>>) -> String {
    get_subformula(nodes, 0)
}