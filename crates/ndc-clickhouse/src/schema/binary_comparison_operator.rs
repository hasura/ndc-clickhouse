use strum::{Display, EnumIter, EnumString};

use crate::sql::ast::{BinaryOperator, Expr, Function};

#[derive(Debug, Clone, EnumString, Display, EnumIter)]
pub enum ClickHouseBinaryComparisonOperator {
    #[strum(to_string = "_eq")]
    Eq,
    #[strum(to_string = "_gt")]
    Gt,
    #[strum(to_string = "_lt")]
    Lt,
    #[strum(to_string = "_gte")]
    GtEq,
    #[strum(to_string = "_lte")]
    LtEq,
    #[strum(to_string = "_neq")]
    NotEq,
    #[strum(to_string = "_like")]
    Like,
    #[strum(to_string = "_nlike")]
    NotLike,
    #[strum(to_string = "_ilike")]
    ILike,
    #[strum(to_string = "_nilike")]
    NotILike,
    #[strum(to_string = "_in")]
    In,
    #[strum(to_string = "_nin")]
    NotIn,
    #[strum(to_string = "_match")]
    Match,
}

impl ClickHouseBinaryComparisonOperator {
    pub fn apply(&self, left: Expr, right: Expr) -> Expr {
        fn apply_operator(op: BinaryOperator, left: Expr, right: Expr) -> Expr {
            Expr::BinaryOp {
                left: left.into_box(),
                op,
                right: right.into_box(),
            }
        }
        fn apply_function(name: &str, left: Expr, right: Expr) -> Expr {
            Function::new_unquoted(name)
                .args(vec![left.into_arg(), right.into_arg()])
                .into_expr()
        }
        use ClickHouseBinaryComparisonOperator as CBO;

        match self {
            CBO::Eq => apply_operator(BinaryOperator::Eq, left, right),
            CBO::Gt => apply_operator(BinaryOperator::Gt, left, right),
            CBO::Lt => apply_operator(BinaryOperator::Lt, left, right),
            CBO::GtEq => apply_operator(BinaryOperator::GtEq, left, right),
            CBO::LtEq => apply_operator(BinaryOperator::LtEq, left, right),
            CBO::NotEq => apply_operator(BinaryOperator::NotEq, left, right),
            CBO::Like => apply_operator(BinaryOperator::Like, left, right),
            CBO::NotLike => apply_operator(BinaryOperator::NotLike, left, right),
            CBO::ILike => apply_operator(BinaryOperator::ILike, left, right),
            CBO::NotILike => apply_operator(BinaryOperator::NotILike, left, right),
            CBO::In => apply_operator(BinaryOperator::In, left, right),
            CBO::NotIn => apply_operator(BinaryOperator::NotIn, left, right),
            CBO::Match => apply_function("match", left, right),
        }
    }
}
