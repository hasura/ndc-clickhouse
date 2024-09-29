use strum::{Display, EnumIter, EnumString};

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
