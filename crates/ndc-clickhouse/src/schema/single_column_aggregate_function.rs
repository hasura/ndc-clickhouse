use strum::{Display, EnumIter, EnumString};

use crate::sql::ast::{Expr, Function};

#[derive(Debug, Clone, EnumString, Display, EnumIter, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum ClickHouseSingleColumnAggregateFunction {
    Max,
    Min,
    Sum,
    Avg,
    StddevPop,
    StddevSamp,
    VarPop,
    VarSamp,
}

impl ClickHouseSingleColumnAggregateFunction {
    pub fn as_expr(&self, column: Expr) -> Expr {
        use ClickHouseSingleColumnAggregateFunction::*;
        let sql_fn = |name: &str, arg: Expr| {
            Function::new_unquoted(name)
                .args(vec![arg.into_arg()])
                .into_expr()
        };
        match self {
            Max => sql_fn("max", column),
            Min => sql_fn("min", column),
            Sum => sql_fn("sum", column),
            Avg => sql_fn("avg", column),
            StddevPop => sql_fn("stddevPop", column),
            StddevSamp => sql_fn("stddevSamp", column),
            VarPop => sql_fn("varPop", column),
            VarSamp => sql_fn("varSamp", column),
        }
    }
}
