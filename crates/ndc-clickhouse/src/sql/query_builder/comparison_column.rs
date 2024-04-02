use common::clickhouse_parser::datatype::ClickHouseDataType;

use crate::sql::ast::{Expr, Function, Ident, Join, Lambda};

use super::and_reducer;

/// A resolved comparison column
/// Contains an identifier that points to the resolved column,
/// and any joins required to access this data
/// To access the column, use the apply function.
/// This function takes a closure as argument, and that closure will receive the column identifier as an argument.
/// If the target column is in another table, the column values are joined as an array of values,
/// and the apply function will wrap the expression returned from the closure with the `arrayExists` function:
/// ```sql
///  arrayExists(
///     (_col_identifier) -> <your expression using _col_identifier here>,
///     _join_alias._array_of_possible_values
///  )
/// ```
/// If the expresssion is true for any of the values in the array, the overall expression is true.
///
/// If the column is _not_ in another table, then the apply function is a no-op and will return the inner expression as-is.
/// If using the same column multiple times, use the apply_expr function to apply the expression without adding the joins,
/// and use the extact_joins function to aquire the joins first.
/// This struct is not clone to avoid issues with collecting the same joins multiple times which would cause a conflict.

#[derive(Debug)]
pub enum ComparisonColumn {
    /// The Simple variant will wrap the expression as-is and returns no joins
    /// For convenience, can be used with any expression as it won't wrap them in the arrayExists function
    Simple {
        column_ident: Expr,
        data_type: ClickHouseDataType,
    },
    /// The Flat variant does not group rows with a subquery. This can be safely used inside exists subqueries,
    /// as duplicating rows there does not matter. Like the Simple variant, when applied we do not wrap the expression in the arrayExists function
    Flat {
        column_ident: Expr,
        joins: Vec<Join>,
        additional_predicate: Option<Expr>,
        data_type: ClickHouseDataType,
    },
    /// The Grouped variant contains a single join which groups all values from the related column into an array.
    /// When applied, the expression returned from the closure is wrapped in the `arrayExists` function, and this expression will be evaluated against all values in the array.
    /// If the expression evaluates to true against any of the values, the overall expression is true and will short-circuit.
    /// Note that grouping very large arrays of values could have performance implications, and we may wish to move away from this in the future, if possible.
    Grouped {
        column_ident: Ident,
        joins: Vec<Join>,
        values_ident: Expr,
        data_type: ClickHouseDataType,
    },
}

impl ComparisonColumn {
    pub fn new_simple(column_ident: Expr, data_type: ClickHouseDataType) -> Self {
        Self::Simple {
            column_ident,
            data_type,
        }
    }
    pub fn new_flat(
        column_ident: Expr,
        joins: Vec<Join>,
        additional_predicate: Option<Expr>,
        data_type: ClickHouseDataType,
    ) -> Self {
        Self::Flat {
            column_ident,
            joins,
            additional_predicate,
            data_type,
        }
    }
    pub fn new_grouped(
        column_ident: Ident,
        join: Join,
        values_ident: Expr,
        data_type: ClickHouseDataType,
    ) -> Self {
        Self::Grouped {
            column_ident,
            joins: vec![join],
            values_ident,
            data_type,
        }
    }
    pub fn data_type(&self) -> ClickHouseDataType {
        match self {
            ComparisonColumn::Simple { data_type, .. }
            | ComparisonColumn::Flat { data_type, .. }
            | ComparisonColumn::Grouped { data_type, .. } => data_type.to_owned(),
        }
    }
    /// consumes self, and wraps an expression and set of joins appropriately.
    /// if self is a simple column, this does nothing and returns the inputs as-is
    /// if self contains any joins, those are added to the set of joins passed as an argument
    pub fn apply<F>(self, use_column: F) -> (Expr, Vec<Join>)
    where
        F: FnOnce(Expr) -> (Expr, Vec<Join>),
    {
        match self {
            ComparisonColumn::Simple {
                column_ident,
                data_type: _,
            } => use_column(column_ident),
            ComparisonColumn::Flat {
                column_ident,
                joins,
                additional_predicate,
                data_type: _,
            } => {
                let (expr, additional_joins) = use_column(column_ident);
                let expr = if let Some(additional_expr) = additional_predicate {
                    and_reducer(expr, additional_expr)
                } else {
                    expr
                };

                (expr, joins.into_iter().chain(additional_joins).collect())
            }
            ComparisonColumn::Grouped {
                column_ident,
                joins,
                values_ident,
                data_type: _,
            } => {
                let (expr, additional_joins) = use_column(column_ident.clone().into_expr());
                let expr = Function::new_unquoted("arrayExists")
                    .args(vec![
                        Lambda::new(vec![column_ident], expr).into_expr().into_arg(),
                        values_ident.into_arg(),
                    ])
                    .into_expr();
                (expr, joins.into_iter().chain(additional_joins).collect())
            }
        }
    }
}
