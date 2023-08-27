use strum::{Display, EnumIter, EnumString};

use crate::sql::ast::BinaryOperator;

#[derive(Debug, Clone, EnumString, Display, EnumIter)]
#[strum(serialize_all = "snake_case")]
pub enum ClickHouseBinaryComparisonOperator {
    Gt,
    Lt,
    GtEq,
    LtEq,
    Eq,
    NotEq,
    Like,
}

impl ClickHouseBinaryComparisonOperator {
    pub fn to_sql_operator(&self) -> BinaryOperator {
        match self {
            ClickHouseBinaryComparisonOperator::Gt => BinaryOperator::Gt,
            ClickHouseBinaryComparisonOperator::Lt => BinaryOperator::Lt,
            ClickHouseBinaryComparisonOperator::GtEq => BinaryOperator::GtEq,
            ClickHouseBinaryComparisonOperator::LtEq => BinaryOperator::LtEq,
            ClickHouseBinaryComparisonOperator::Eq => BinaryOperator::Eq,
            ClickHouseBinaryComparisonOperator::NotEq => BinaryOperator::NotEq,
            ClickHouseBinaryComparisonOperator::Like => BinaryOperator::Like,
        }
    }
}
