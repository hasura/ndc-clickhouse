use crate::sql::ast::{Expr, Function};
use common::schema::single_column_aggregate_function::ClickHouseSingleColumnAggregateFunction;

pub fn apply_function(function: &ClickHouseSingleColumnAggregateFunction, column: Expr) -> Expr {
    use ClickHouseSingleColumnAggregateFunction::*;
    let sql_fn = |name: &str, arg: Expr| {
        Function::new_unquoted(name)
            .args(vec![arg.into_arg()])
            .into_expr()
    };
    match function {
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
