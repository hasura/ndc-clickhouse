use strum::{Display, EnumIter, EnumString};

use crate::sql::ast::{BinaryOperator, Expr, Function};

#[derive(Debug, Clone, EnumString, Display, EnumIter)]
pub enum ClickHouseBinaryComparisonOperator {
    #[strum(serialize = "_gt")]
    Gt,
    #[strum(serialize = "_lt")]
    Lt,
    #[strum(serialize = "_gte")]
    GtEq,
    #[strum(serialize = "_lte")]
    LtEq,
    #[strum(serialize = "_neq")]
    NotEq,
    #[strum(serialize = "_like")]
    Like,
    #[strum(serialize = "_nlike")]
    NotLike,
    #[strum(serialize = "_ilike")]
    ILike,
    #[strum(serialize = "_nilike")]
    NotILike,
    #[strum(serialize = "_match")]
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
            CBO::Gt => apply_operator(BinaryOperator::Gt, left, right),
            CBO::Lt => apply_operator(BinaryOperator::Lt, left, right),
            CBO::GtEq => apply_operator(BinaryOperator::GtEq, left, right),
            CBO::LtEq => apply_operator(BinaryOperator::LtEq, left, right),
            CBO::NotEq => apply_operator(BinaryOperator::NotEq, left, right),
            CBO::Like => apply_operator(BinaryOperator::Like, left, right),
            CBO::NotLike => apply_operator(BinaryOperator::NotLike, left, right),
            CBO::ILike => apply_operator(BinaryOperator::ILike, left, right),
            CBO::NotILike => apply_operator(BinaryOperator::NotILike, left, right),
            CBO::Match => apply_function("match", left, right),
        }
    }
}
