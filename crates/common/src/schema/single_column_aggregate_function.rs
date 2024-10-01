use strum::{Display, EnumIter, EnumString};

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
