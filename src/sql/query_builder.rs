use std::str::FromStr;

use indexmap::IndexMap;
use ndc_sdk::models;

mod comparison_column;
mod error;
mod typecasting;

use comparison_column::ComparisonColumn;
pub use error::QueryBuilderError;
use typecasting::{AggregatesTypeString, RowsTypeString};

use super::ast::*;
use crate::{
    connector::config::ServerConfig,
    schema::{ClickHouseBinaryComparisonOperator, ClickHouseSingleColumnAggregateFunction},
};

pub struct QueryBuilder<'r, 'c> {
    request: &'r models::QueryRequest,
    configuration: &'c ServerConfig,
}

impl<'r, 'c> QueryBuilder<'r, 'c> {
    pub fn new(request: &'r models::QueryRequest, configuration: &'c ServerConfig) -> Self {
        Self {
            request,
            configuration,
        }
    }
    pub fn build(&self) -> Result<UnsafeInlinedStatement, QueryBuilderError> {
        self.root_query()
    }
    fn rows_typecast_string(
        &self,
        fields: &IndexMap<String, models::Field>,
        collection_alias: &str,
    ) -> Result<String, QueryBuilderError> {
        Ok(RowsTypeString::new(
            collection_alias,
            fields,
            &self.request.collection_relationships,
            self.configuration,
        )
        .map_err(|err| QueryBuilderError::Typecasting(err.to_string()))?
        .to_string())
    }
    fn agregates_typecast_string(
        &self,
        aggregates: &IndexMap<String, models::Aggregate>,
        collection_alias: &str,
    ) -> Result<String, QueryBuilderError> {
        Ok(
            AggregatesTypeString::new(collection_alias, aggregates, self.configuration)
                .map_err(|err| QueryBuilderError::Typecasting(err.to_string()))?
                .to_string(),
        )
    }
    fn root_query(&self) -> Result<UnsafeInlinedStatement, QueryBuilderError> {
        let collection = &self.request.collection;
        let query = &self.request.query;

        let get_typecasting_wrapper = |index: usize, alias: &str, typecast_string: String| {
            Function::new_unquoted("cast")
                .args(vec![
                    Function::new_unquoted("tupleElement")
                        .args(vec![
                            Expr::CompoundIdentifier(vec![
                                Ident::new_quoted("_rowset"),
                                Ident::new_quoted("_rowset"),
                            ])
                            .into_arg(),
                            Expr::Value(Value::Number(index.to_string())).into_arg(),
                        ])
                        .into_expr()
                        .into_arg(),
                    Expr::Value(Value::SingleQuotedString(typecast_string)).into_arg(),
                ])
                .into_expr()
                .into_select(Some(alias))
        };

        let select = match (&self.request.query.fields, &self.request.query.aggregates) {
            (None, None) => vec![Expr::Value(Value::Null).into_select::<String>(None)],
            (None, Some(aggregates)) => vec![get_typecasting_wrapper(
                1,
                "aggregates",
                self.agregates_typecast_string(aggregates, collection)?,
            )],
            (Some(fields), None) => vec![get_typecasting_wrapper(
                1,
                "rows",
                self.rows_typecast_string(fields, collection)?,
            )],
            (Some(fields), Some(aggregates)) => vec![
                get_typecasting_wrapper(1, "rows", self.rows_typecast_string(fields, collection)?),
                get_typecasting_wrapper(
                    2,
                    "aggregates",
                    self.agregates_typecast_string(aggregates, collection)?,
                ),
            ],
        };

        let with = if let Some(variables) = &self.request.variables {
            let mut variable_values: IndexMap<String, Vec<serde_json::Value>> = IndexMap::new();

            variable_values.insert(
                "_varset_id".to_string(),
                (1..=variables.len()).map(serde_json::Value::from).collect(),
            );

            for varset in variables {
                for (varkey, varvalue) in varset {
                    let varkey = format!("_var_{varkey}");

                    if let Some(varkeyvalues) = variable_values.get_mut(&varkey) {
                        varkeyvalues.push(varvalue.clone());
                    } else {
                        variable_values.insert(varkey.to_owned(), vec![varvalue.clone()]);
                    }
                }
            }

            let variables_values =
                Parameter::new(
                    Value::SingleQuotedString(serde_json::to_string(&variable_values).map_err(
                        |err| QueryBuilderError::CannotSerializeVariables(err.to_string()),
                    )?),
                    "String".to_owned(),
                )
                .into_expr();

            vec![Query::default()
                .select(vec![SelectItem::Wildcard])
                .from(vec![Function::new_unquoted("format")
                    .args(vec![
                        Expr::Identifier(Ident::new_unquoted("JSONColumns")).into_arg(),
                        variables_values.into_arg(),
                    ])
                    .into_table_factor()
                    .into_table_with_joins(vec![])])
                .into_with_item("_vars")]
        } else {
            vec![]
        };

        let from = vec![self
            .rowset_subquery(collection, &vec![], query)?
            .into_table_factor()
            .alias("_rowset")
            .into_table_with_joins(vec![])];

        let order_by = if self.request.variables.is_some() {
            vec![OrderByExpr {
                expr: Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_rowset"),
                    Ident::new_quoted("_varset_id"),
                ]),
                asc: Some(true),
                nulls_first: None,
            }]
        } else {
            vec![]
        };

        Ok(Query::new()
            .with(with)
            .select(select)
            .from(from)
            .order_by(order_by)
            .into_statement()
            .format("JSON"))
    }
    fn rowset_subquery(
        &self,
        current_collection: &str,
        relkeys: &Vec<&String>,
        query: &models::Query,
    ) -> Result<Query, QueryBuilderError> {
        let fields = if let Some(fields) = &query.fields {
            let row = if fields.is_empty() {
                Function::new_unquoted("map").into_expr()
            } else {
                let args = fields
                    .iter()
                    .map(|(alias, _field)| {
                        Expr::CompoundIdentifier(vec![
                            Ident::new_quoted("_row"),
                            Ident::new_quoted(format!("_field_{alias}")),
                        ])
                        .into_arg()
                    })
                    .collect();
                Function::new_unquoted("tuple").args(args).into_expr()
            }
            .into_arg();
            Some(
                Function::new_unquoted("groupArray")
                    .args(vec![row])
                    .into_expr(),
            )
        } else {
            None
        };

        let aggregates = if let Some(aggregates) = &query.aggregates {
            Some(if aggregates.is_empty() {
                Function::new_unquoted("map").into_expr()
            } else {
                let args = aggregates
                    .iter()
                    .map(|(alias, aggregate)| {
                        Ok(match aggregate {
                            models::Aggregate::StarCount {} => Function::new_unquoted("COUNT")
                                .args(vec![FunctionArgExpr::Wildcard])
                                .into_expr(),
                            models::Aggregate::ColumnCount {
                                distinct,
                                column: _,
                            } => {
                                let column = Expr::CompoundIdentifier(vec![
                                    Ident::new_quoted("_row"),
                                    Ident::new_quoted(format!("_agg_{alias}")),
                                ]);
                                Function::new_unquoted("COUNT")
                                    .args(vec![column.into_arg()])
                                    .distinct(*distinct)
                                    .into_expr()
                            }
                            models::Aggregate::SingleColumn {
                                function,
                                column: _,
                            } => {
                                let column = Expr::CompoundIdentifier(vec![
                                    Ident::new_quoted("_row"),
                                    Ident::new_quoted(format!("_agg_{alias}")),
                                ]);
                                aggregate_function(function)?.as_expr(column)
                            }
                        }
                        .into_arg())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Function::new_unquoted("tuple").args(args).into_expr()
            })
        } else {
            None
        };

        let rowset = match (fields, aggregates) {
            (None, None) => Function::new_unquoted("map"),
            (None, Some(aggregates)) => {
                Function::new_unquoted("tuple").args(vec![aggregates.into_arg()])
            }
            (Some(fields), None) => Function::new_unquoted("tuple").args(vec![fields.into_arg()]),
            (Some(fields), Some(aggregates)) => {
                Function::new_unquoted("tuple").args(vec![fields.into_arg(), aggregates.into_arg()])
            }
        }
        .into_expr()
        .into_select(Some("_rowset"));

        let mut select = vec![rowset];
        let mut group_by = vec![];

        for relkey in relkeys {
            let relkey = format!("_relkey_{relkey}");
            let col = Expr::CompoundIdentifier(vec![
                Ident::new_quoted("_row"),
                Ident::new_quoted(&relkey),
            ]);
            select.push(col.clone().into_select(Some(&relkey)));
            group_by.push(col);
        }

        if self.request.variables.is_some() {
            let col = Expr::CompoundIdentifier(vec![
                Ident::new_quoted("_row"),
                Ident::new_quoted("_varset_id"),
            ]);
            select.push(col.clone().into_select(Some("_varset_id")));
            group_by.push(col);
        }

        let from = vec![self
            .row_subquery(current_collection, relkeys, query)?
            .into_table_factor()
            .alias("_row")
            .into_table_with_joins(vec![])];

        Ok(Query::new().select(select).from(from).group_by(group_by))
    }
    fn row_subquery(
        &self,
        current_collection: &str,
        relkeys: &Vec<&String>,
        query: &models::Query,
    ) -> Result<Query, QueryBuilderError> {
        let mut select = vec![];

        if let Some(fields) = &query.fields {
            for (alias, field) in fields {
                let expr = match field {
                    models::Field::Column { column, fields } => {
                        if fields.is_some() {
                            todo!("support nested field selection")
                        }
                        Expr::CompoundIdentifier(vec![
                            Ident::new_quoted("_origin"),
                            self.column_ident(column, current_collection)?,
                        ])
                    }
                    models::Field::Relationship { .. } => Expr::CompoundIdentifier(vec![
                        Ident::new_quoted(format!("_rel_{alias}")),
                        Ident::new_quoted("_rowset"),
                    ]),
                };
                select.push(expr.into_select(Some(format!("_field_{alias}"))))
            }
        }

        if let Some(aggregates) = &query.aggregates {
            for (alias, aggregate) in aggregates {
                if let models::Aggregate::ColumnCount { column, .. }
                | models::Aggregate::SingleColumn { column, .. } = aggregate
                {
                    let expr = Expr::CompoundIdentifier(vec![
                        Ident::new_quoted("_origin"),
                        self.column_ident(column, current_collection)?,
                    ]);
                    select.push(expr.into_select(Some(format!("_agg_{alias}"))))
                }
            }
        }

        for relkey in relkeys {
            select.push(
                Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_origin"),
                    self.column_ident(relkey, current_collection)?,
                ])
                .into_select(Some(format!("_relkey_{relkey}"))),
            )
        }

        if self.request.variables.is_some() {
            select.push(
                Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_vars"),
                    Ident::new_quoted("_varset_id"),
                ])
                .into_select(Some("_varset_id")),
            )
        }

        if select.is_empty() {
            select.push(Expr::Value(Value::Null).into_select::<String>(None))
        }

        let (table, mut base_joins) = if self.request.variables.is_some() {
            let table = ObjectName(vec![Ident::new_quoted("_vars")])
                .into_table_factor()
                .alias("_vars");

            let joins = vec![Join {
                relation: self
                    .collection_ident(current_collection)?
                    .into_table_factor()
                    .alias("_origin"),
                join_operator: JoinOperator::CrossJoin,
            }];
            (table, joins)
        } else {
            let table = self
                .collection_ident(current_collection)?
                .into_table_factor()
                .alias("_origin");
            (table, vec![])
        };

        if let Some(fields) = &query.fields {
            for (alias, field) in fields {
                if let models::Field::Relationship {
                    query,
                    relationship,
                    arguments: _,
                } = field
                {
                    let relationship = self.collection_relationship(relationship)?;

                    let mut join_expr = relationship
                        .column_mapping
                        .iter()
                        .map(|(source_col, target_col)| {
                            Ok(Expr::BinaryOp {
                                left: Expr::CompoundIdentifier(vec![
                                    Ident::new_quoted("_origin"),
                                    self.column_ident(source_col, current_collection)?,
                                ])
                                .into_box(),
                                op: BinaryOperator::Eq,
                                right: Expr::CompoundIdentifier(vec![
                                    Ident::new_quoted(format!("_rel_{alias}")),
                                    Ident::new_quoted(format!("_relkey_{target_col}")),
                                ])
                                .into_box(),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    if self.request.variables.is_some() {
                        join_expr.push(Expr::BinaryOp {
                            left: Expr::CompoundIdentifier(vec![
                                Ident::new_quoted("_vars"),
                                Ident::new_quoted("_varset_id"),
                            ])
                            .into_box(),
                            op: BinaryOperator::Eq,
                            right: Expr::CompoundIdentifier(vec![
                                Ident::new_quoted(format!("_rel_{alias}")),
                                Ident::new_quoted("_varset_id"),
                            ])
                            .into_box(),
                        })
                    }

                    let join_operator = join_expr
                        .into_iter()
                        .reduce(and_reducer)
                        .map(|expr| JoinOperator::LeftOuter(JoinConstraint::On(expr)))
                        .unwrap_or(JoinOperator::CrossJoin);

                    let relkeys = relationship.column_mapping.values().collect();

                    let join = Join {
                        relation: self
                            .rowset_subquery(&relationship.target_collection, &relkeys, query)?
                            .into_table_factor()
                            .alias(format!("_rel_{alias}")),
                        join_operator,
                    };

                    base_joins.push(join)
                }
            }
        }

        let (predicate, predicate_joins) = if let Some(predicate) = &query.predicate {
            self.filter_expression(
                predicate,
                &Ident::new_quoted("_origin"),
                current_collection,
                true,
                &mut 0,
            )
            .map(|(expr, joins)| (Some(expr), joins))?
        } else {
            (None, vec![])
        };

        let mut order_by_exprs = vec![];
        let mut order_by_joins = vec![];

        if let Some(order_by) = &query.order_by {
            let mut order_by_index = 0;

            for element in &order_by.elements {
                match &element.target {
                    models::OrderByTarget::Column { name, path } if path.is_empty() => {
                        let expr = Expr::CompoundIdentifier(vec![
                            Ident::new_quoted("_origin"),
                            self.column_ident(name, current_collection)?,
                        ]);
                        let asc = match &element.order_direction {
                            models::OrderDirection::Asc => Some(true),
                            models::OrderDirection::Desc => Some(false),
                        };
                        order_by_exprs.push(OrderByExpr {
                            expr,
                            asc,
                            nulls_first: None,
                        })
                    }
                    models::OrderByTarget::Column { path, .. }
                    | models::OrderByTarget::SingleColumnAggregate { path, .. }
                    | models::OrderByTarget::StarCountAggregate { path } => {
                        let join_alias = Ident::new_quoted(format!("_order_by_{order_by_index}"));
                        order_by_index += 1;

                        let first_element = path.first().ok_or(QueryBuilderError::Unexpected(
                            "expected order by path to have at least one element".to_string(),
                        ))?;

                        let relationship =
                            self.collection_relationship(&first_element.relationship)?;

                        let subquery = {
                            let mut select = vec![];
                            let mut group_by = vec![];
                            let mut limit_by = vec![];

                            let join_alias = Ident::new_quoted("_order_by_0");

                            for target_col in relationship.column_mapping.values() {
                                select.push(
                                    Expr::CompoundIdentifier(vec![
                                        join_alias.clone(),
                                        self.column_ident(
                                            target_col,
                                            &relationship.target_collection,
                                        )?,
                                    ])
                                    .into_select(Some(format!("_relkey_{target_col}"))),
                                );
                                group_by.push(Expr::CompoundIdentifier(vec![
                                    join_alias.clone(),
                                    self.column_ident(target_col, &relationship.target_collection)?,
                                ]));
                                limit_by.push(Expr::CompoundIdentifier(vec![
                                    join_alias.clone(),
                                    self.column_ident(target_col, &relationship.target_collection)?,
                                ]));
                            }

                            if self.request.variables.is_some() {
                                select.push(
                                    Expr::CompoundIdentifier(vec![
                                        Ident::new_quoted("_vars"),
                                        Ident::new_quoted("_varset_id"),
                                    ])
                                    .into_select(Some("_varset_id")),
                                );
                                group_by.push(Expr::CompoundIdentifier(vec![
                                    Ident::new_quoted("_vars"),
                                    Ident::new_quoted("_varset_id"),
                                ]));
                                limit_by.push(Expr::CompoundIdentifier(vec![
                                    Ident::new_quoted("_vars"),
                                    Ident::new_quoted("_varset_id"),
                                ]));
                            }

                            let table = self
                                .collection_ident(&relationship.target_collection)?
                                .into_table_factor()
                                .alias(&join_alias);

                            let (table, base_joins) = if self.request.variables.is_some() {
                                (
                                    ObjectName(vec![Ident::new_quoted("_vars")])
                                        .into_table_factor(),
                                    vec![Join {
                                        relation: table,
                                        join_operator: JoinOperator::CrossJoin,
                                    }],
                                )
                            } else {
                                (table, vec![])
                            };

                            let mut join_index = 1;

                            let mut additional_joins = vec![];
                            let mut additional_predicate = vec![];

                            if let Some(expression) = &first_element.predicate {
                                let (predicate, predicate_joins) = self.filter_expression(
                                    expression,
                                    &join_alias,
                                    &relationship.target_collection,
                                    false,
                                    &mut join_index,
                                )?;

                                additional_predicate.push(predicate);

                                for predicate_join in predicate_joins {
                                    additional_joins.push(predicate_join);
                                }
                            }

                            let mut last_join_alias = join_alias;
                            let mut last_collection_name = &relationship.target_collection;

                            for path_element in path.iter().skip(1) {
                                let join_alias =
                                    Ident::new_quoted(format!("_order_by_{join_index}"));
                                join_index += 1;

                                let relationship =
                                    self.collection_relationship(&path_element.relationship)?;

                                let join_exprs = relationship
                                    .column_mapping
                                    .iter()
                                    .map(|(source_col, target_col)| {
                                        Ok(Expr::BinaryOp {
                                            left: Expr::CompoundIdentifier(vec![
                                                last_join_alias.clone(),
                                                self.column_ident(
                                                    source_col,
                                                    last_collection_name,
                                                )?,
                                            ])
                                            .into_box(),
                                            op: BinaryOperator::Eq,
                                            right: Expr::CompoundIdentifier(vec![
                                                join_alias.clone(),
                                                self.column_ident(
                                                    target_col,
                                                    &relationship.target_collection,
                                                )?,
                                            ])
                                            .into_box(),
                                        })
                                    })
                                    .collect::<Result<Vec<_>, _>>()?;

                                let join_operator = join_exprs
                                    .into_iter()
                                    .reduce(and_reducer)
                                    .map(JoinConstraint::On)
                                    .map(JoinOperator::Inner)
                                    .unwrap_or(JoinOperator::CrossJoin);

                                let relation = self
                                    .collection_ident(&relationship.target_collection)?
                                    .into_table_factor()
                                    .alias(&join_alias);

                                let join = Join {
                                    relation,
                                    join_operator,
                                };

                                additional_joins.push(join);

                                if let Some(expression) = &path_element.predicate {
                                    let (predicate, predicate_joins) = self.filter_expression(
                                        expression,
                                        &join_alias,
                                        &relationship.target_collection,
                                        false,
                                        &mut join_index,
                                    )?;

                                    additional_predicate.push(predicate);

                                    for predicate_join in predicate_joins {
                                        additional_joins.push(predicate_join);
                                    }
                                }

                                last_join_alias = join_alias;
                                last_collection_name = &relationship.target_collection;
                            }

                            match &element.target {
                                models::OrderByTarget::Column { name, path: _ } => {
                                    let column = Expr::CompoundIdentifier(vec![
                                        last_join_alias,
                                        self.column_ident(name, last_collection_name)?,
                                    ]);
                                    select.push(column.into_select(Some("_order_by_value")))
                                }
                                models::OrderByTarget::SingleColumnAggregate {
                                    column,
                                    function,
                                    path: _,
                                } => {
                                    let column = Expr::CompoundIdentifier(vec![
                                        last_join_alias,
                                        self.column_ident(column, last_collection_name)?,
                                    ]);
                                    select.push(
                                        aggregate_function(function)?
                                            .as_expr(column)
                                            .into_select(Some("_order_by_value")),
                                    )
                                }
                                models::OrderByTarget::StarCountAggregate { path: _ } => {
                                    if select.is_empty() {
                                        select.push(
                                            Expr::Value(Value::Number("1".to_string()))
                                                .into_select(Some("_order_by_value")),
                                        )
                                    }
                                }
                            }

                            let joins = base_joins
                                .into_iter()
                                .chain(additional_joins.into_iter())
                                .collect();

                            let from = vec![table.into_table_with_joins(joins)];

                            let predicate = additional_predicate.into_iter().reduce(and_reducer);

                            let limit_by = Some(LimitByExpr::new(Some(1), None, limit_by));

                            Query::new()
                                .select(select)
                                .from(from)
                                .predicate(predicate)
                                .group_by(group_by)
                                .limit_by(limit_by)
                        };

                        let join_operator = {
                            let mut join_exprs = relationship
                                .column_mapping
                                .iter()
                                .map(|(source_col, target_col)| {
                                    Ok(Expr::BinaryOp {
                                        left: Expr::CompoundIdentifier(vec![
                                            Ident::new_quoted("_origin"),
                                            self.column_ident(source_col, &current_collection)?,
                                        ])
                                        .into_box(),
                                        op: BinaryOperator::Eq,
                                        right: Expr::CompoundIdentifier(vec![
                                            join_alias.clone(),
                                            Ident::new_quoted(format!("_relkey_{target_col}")),
                                        ])
                                        .into_box(),
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()?;

                            if self.request.variables.is_some() {
                                join_exprs.push(Expr::BinaryOp {
                                    left: Expr::CompoundIdentifier(vec![
                                        Ident::new_quoted("_vars"),
                                        Ident::new_quoted("_varset_id"),
                                    ])
                                    .into_box(),
                                    op: BinaryOperator::Eq,
                                    right: Expr::CompoundIdentifier(vec![
                                        join_alias.clone(),
                                        Ident::new_quoted("_varset_id"),
                                    ])
                                    .into_box(),
                                })
                            }

                            join_exprs
                                .into_iter()
                                .reduce(and_reducer)
                                .map(JoinConstraint::On)
                                .map(JoinOperator::LeftOuter)
                                .unwrap_or(JoinOperator::CrossJoin)
                        };

                        let join = Join {
                            relation: subquery.into_table_factor().alias(join_alias.clone()),
                            join_operator,
                        };

                        order_by_joins.push(join);

                        let expr = Expr::CompoundIdentifier(vec![
                            join_alias,
                            Ident::new_quoted("_order_by_value"),
                        ]);
                        let asc = match &element.order_direction {
                            models::OrderDirection::Asc => Some(true),
                            models::OrderDirection::Desc => Some(false),
                        };
                        order_by_exprs.push(OrderByExpr {
                            expr,
                            asc,
                            nulls_first: None,
                        })
                    }
                }
            }
        }

        let joins = base_joins
            .into_iter()
            .chain(predicate_joins)
            .chain(order_by_joins)
            .collect();

        let from = vec![table.into_table_with_joins(joins)];

        let mut limit_by_cols = relkeys
            .iter()
            .map(|relkey| {
                Ok(Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_origin"),
                    self.column_ident(relkey, current_collection)?,
                ]))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if self.request.variables.is_some() {
            limit_by_cols.push(Expr::CompoundIdentifier(vec![
                Ident::new_quoted("_vars"),
                Ident::new_quoted("_varset_id"),
            ]));
        }

        let (limit_by, limit, offset) = if limit_by_cols.is_empty() {
            (
                None,
                query.limit.map(|limit| limit as u64),
                query.offset.map(|offset| offset as u64),
            )
        } else {
            let limit_by = match (query.limit, query.offset) {
                (None, None) => None,
                (None, Some(offset)) => {
                    Some(LimitByExpr::new(None, Some(offset as u64), limit_by_cols))
                }
                (Some(limit), None) => {
                    Some(LimitByExpr::new(Some(limit as u64), None, limit_by_cols))
                }
                (Some(limit), Some(offset)) => Some(LimitByExpr::new(
                    Some(limit as u64),
                    Some(offset as u64),
                    limit_by_cols,
                )),
            };

            (limit_by, None, None)
        };

        Ok(Query::new()
            .select(select)
            .from(from)
            .predicate(predicate)
            .order_by(order_by_exprs)
            .limit_by(limit_by)
            .limit(limit)
            .offset(offset))
    }
    fn filter_expression(
        &self,
        expression: &models::Expression,
        current_join_alias: &Ident,
        current_collection: &str,
        current_is_origin: bool,
        name_index: &mut u32,
    ) -> Result<(Expr, Vec<Join>), QueryBuilderError> {
        match expression {
            models::Expression::And { expressions } => {
                let (and_expressions, joins): (Vec<_>, Vec<_>) = expressions
                    .iter()
                    .map(|expression| {
                        self.filter_expression(
                            expression,
                            current_join_alias,
                            current_collection,
                            current_is_origin,
                            name_index,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .unzip();

                let joins: Vec<_> = joins.into_iter().flatten().collect();

                let and_expression = and_expressions
                    .into_iter()
                    .reduce(and_reducer)
                    .unwrap_or_else(|| Expr::Value(Value::Boolean(true)));

                let and_expression = if expressions.len() > 1 {
                    and_expression.into_nested()
                } else {
                    and_expression
                };

                Ok((and_expression, joins))
            }
            models::Expression::Or { expressions } => {
                let (or_expressions, joins): (Vec<_>, Vec<_>) = expressions
                    .iter()
                    .map(|expression| {
                        self.filter_expression(
                            expression,
                            current_join_alias,
                            current_collection,
                            current_is_origin,
                            name_index,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .unzip();

                let joins: Vec<_> = joins.into_iter().flatten().collect();

                let or_expression = or_expressions
                    .into_iter()
                    .reduce(or_reducer)
                    .unwrap_or_else(|| Expr::Value(Value::Boolean(false)));

                let or_expression = if expressions.len() > 1 {
                    or_expression.into_nested()
                } else {
                    or_expression
                };

                Ok((or_expression, joins))
            }
            models::Expression::Not { expression } => {
                let (expression, joins) = self.filter_expression(
                    expression,
                    current_join_alias,
                    current_collection,
                    current_is_origin,
                    name_index,
                )?;
                let not_expression = Expr::Not(expression.into_nested().into_box());
                Ok((not_expression, joins))
            }
            models::Expression::UnaryComparisonOperator { column, operator } => {
                let left_col = self.comparison_column(
                    column,
                    current_join_alias,
                    current_collection,
                    current_is_origin,
                    name_index,
                )?;

                let (expression, joins) = left_col.apply(|left_col| {
                    let expr = match operator {
                        models::UnaryComparisonOperator::IsNull => Expr::BinaryOp {
                            left: left_col.into_nested().into_box(),
                            op: BinaryOperator::Is,
                            right: Value::Null.into_expr().into_box(),
                        },
                    };
                    (expr, vec![])
                });

                Ok((expression, joins))
            }
            models::Expression::BinaryComparisonOperator {
                column,
                operator,
                value,
            } => {
                let operator =
                    ClickHouseBinaryComparisonOperator::from_str(operator).map_err(|_err| {
                        QueryBuilderError::UnknownBinaryComparisonOperator(operator.to_owned())
                    })?;

                let left_col = self.comparison_column(
                    column,
                    current_join_alias,
                    current_collection,
                    current_is_origin,
                    name_index,
                )?;

                // special case: right hand data types is assumed to always be the same type as left hand,
                // except when the operator is IN/NOT IN, where the type is Array(<left hand data type>)
                let right_col_type = match operator {
                    ClickHouseBinaryComparisonOperator::In
                    | ClickHouseBinaryComparisonOperator::NotIn => {
                        format!("Array({})", left_col.data_type())
                    }
                    _ => left_col.data_type(),
                };

                let right_col = match value {
                    models::ComparisonValue::Column { column } => self.comparison_column(
                        column,
                        current_join_alias,
                        current_collection,
                        current_is_origin,
                        name_index,
                    )?,
                    models::ComparisonValue::Scalar { value } => ComparisonColumn::new_simple(
                        Parameter::new(value.into(), right_col_type.clone()).into_expr(),
                        right_col_type,
                    ),

                    models::ComparisonValue::Variable { name } => ComparisonColumn::new_simple(
                        Expr::CompoundIdentifier(vec![
                            Ident::new_quoted("_vars"),
                            Ident::new_quoted(format!("_var_{name}")),
                        ]),
                        right_col_type,
                    ),
                };

                let (expression, expression_joins) = right_col.apply(|right_col| {
                    left_col.apply(|left_col| {
                        let expression = operator.apply(left_col, right_col);
                        (expression, vec![])
                    })
                });

                Ok((expression, expression_joins))
            }
            models::Expression::Exists {
                in_collection,
                predicate,
            } => self.filter_exists_expression(
                in_collection,
                predicate,
                current_join_alias,
                current_collection,
                name_index,
            ),
        }
    }
    fn filter_exists_expression(
        &self,
        in_collection: &models::ExistsInCollection,
        expression: &Option<Box<models::Expression>>,
        previous_join_alias: &Ident,
        previous_collection: &str,
        name_index: &mut u32,
    ) -> Result<(Expr, Vec<Join>), QueryBuilderError> {
        let exists_join_ident = Ident::new_quoted(format!("_exists_{}", name_index));
        *name_index += 1;

        let join_subquery = {
            let target_collection = match in_collection {
                models::ExistsInCollection::Related {
                    relationship,
                    arguments: _,
                } => {
                    let relationship = self.collection_relationship(relationship)?;
                    &relationship.target_collection
                }
                models::ExistsInCollection::Unrelated {
                    collection,
                    arguments: _,
                } => collection,
            };

            let subquery_origin_alias = Ident::new_quoted(format!("_exists_{}", name_index));
            *name_index += 1;

            let (predicate, predicate_joins) = match expression {
                Some(expression) => {
                    let (predicate, predicate_joins) = self.filter_expression(
                        expression,
                        &subquery_origin_alias,
                        target_collection,
                        false,
                        name_index,
                    )?;
                    (Some(predicate), predicate_joins)
                }
                None => (None, vec![]),
            };

            let table = self
                .collection_ident(target_collection)?
                .into_table_factor()
                .alias(&subquery_origin_alias);

            let (table, base_joins) = if self.request.variables.is_some() {
                (
                    ObjectName(vec![Ident::new_quoted("_vars")]).into_table_factor(),
                    vec![Join {
                        relation: table,
                        join_operator: JoinOperator::CrossJoin,
                    }],
                )
            } else {
                (table, vec![])
            };

            let joins = base_joins.into_iter().chain(predicate_joins).collect();

            let from = vec![table.into_table_with_joins(joins)];

            let mut select =
                vec![Expr::Value(Value::Boolean(true)).into_select(Some(&exists_join_ident))];
            let mut limit_by = vec![];

            if let models::ExistsInCollection::Related {
                relationship,
                arguments: _,
            } = in_collection
            {
                let relationship = self.collection_relationship(relationship)?;

                for target_col in relationship.column_mapping.values() {
                    select.push(
                        Expr::CompoundIdentifier(vec![
                            subquery_origin_alias.clone(),
                            self.column_ident(target_col, target_collection)?,
                        ])
                        .into_select(Some(format!("_relkey_{target_col}"))),
                    );
                    limit_by.push(Expr::CompoundIdentifier(vec![
                        subquery_origin_alias.clone(),
                        self.column_ident(target_col, target_collection)?,
                    ]));
                }
            }

            if self.request.variables.is_some() {
                select.push(
                    Expr::CompoundIdentifier(vec![
                        Ident::new_quoted("_vars"),
                        Ident::new_quoted("_varset_id"),
                    ])
                    .into_select(Some("_varset_id")),
                );
                limit_by.push(Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_vars"),
                    Ident::new_quoted("_varset_id"),
                ]));
            }

            let limit = if limit_by.is_empty() { Some(1) } else { None };
            let limit_by = if !limit_by.is_empty() {
                Some(LimitByExpr::new(Some(1), None, limit_by))
            } else {
                None
            };

            Query::new()
                .select(select)
                .from(from)
                .predicate(predicate)
                .limit(limit)
                .limit_by(limit_by)
        };

        let mut join_exprs = match in_collection {
            models::ExistsInCollection::Related {
                relationship,
                arguments: _,
            } => {
                let relationship = self.collection_relationship(relationship)?;

                relationship
                    .column_mapping
                    .iter()
                    .map(|(source_col, target_col)| {
                        let left = Expr::CompoundIdentifier(vec![
                            previous_join_alias.clone(),
                            self.column_ident(source_col, previous_collection)?,
                        ])
                        .into_box();
                        let right = Expr::CompoundIdentifier(vec![
                            exists_join_ident.clone(),
                            Ident::new_quoted(format!("_relkey_{target_col}")),
                        ])
                        .into_box();
                        Ok(Expr::BinaryOp {
                            left,
                            op: BinaryOperator::Eq,
                            right,
                        })
                    })
                    .collect::<Result<_, _>>()?
            }
            models::ExistsInCollection::Unrelated {
                collection: _,
                arguments: _,
            } => vec![],
        };

        if self.request.variables.is_some() {
            let left = Expr::CompoundIdentifier(vec![
                Ident::new_quoted("_vars"),
                Ident::new_quoted("_varset_id"),
            ])
            .into_box();
            let right = Expr::CompoundIdentifier(vec![
                exists_join_ident.clone(),
                Ident::new_quoted("_varset_id"),
            ])
            .into_box();
            let expr = Expr::BinaryOp {
                left,
                op: BinaryOperator::Eq,
                right,
            };
            join_exprs.push(expr)
        }

        let join_operator = join_exprs
            .into_iter()
            .reduce(and_reducer)
            .map(JoinConstraint::On)
            .map(JoinOperator::LeftOuter)
            .unwrap_or(JoinOperator::CrossJoin);

        let join = Join {
            relation: join_subquery
                .into_table_factor()
                .alias(exists_join_ident.clone()),
            join_operator,
        };

        let expr = Expr::BinaryOp {
            left: Expr::CompoundIdentifier(vec![exists_join_ident.clone(), exists_join_ident])
                .into_box(),
            op: BinaryOperator::Eq,
            right: Expr::Value(Value::Boolean(true)).into_box(),
        };

        Ok((expr, vec![join]))
    }
    fn comparison_column(
        &self,
        column: &models::ComparisonTarget,
        current_join_alias: &Ident,
        current_collection: &str,
        current_is_origin: bool,
        name_index: &mut u32,
    ) -> Result<ComparisonColumn, QueryBuilderError> {
        match column {
            models::ComparisonTarget::Column {
                name: comparison_column_name,
                path,
            } => {
                if let Some(first_element) = path.first() {
                    if current_is_origin {
                        let (join, join_alias, last_collection_name) = {
                            let previous_join_alias = current_join_alias.clone();
                            let current_join_alias =
                                Ident::new_quoted(format!("_exists_{name_index}"));
                            *name_index += 1;

                            let relationship =
                                self.collection_relationship(&first_element.relationship)?;

                            let (subquery, last_collection_name) = {
                                let mut select = vec![];
                                let mut group_by = vec![];

                                let join_alias = Ident::new_quoted("_exists_0");

                                for target_col in relationship.column_mapping.values() {
                                    select.push(
                                        Expr::CompoundIdentifier(vec![
                                            join_alias.clone(),
                                            self.column_ident(
                                                target_col,
                                                &relationship.target_collection,
                                            )?,
                                        ])
                                        .into_select(Some(format!("_relkey_{target_col}"))),
                                    );
                                    group_by.push(Expr::CompoundIdentifier(vec![
                                        join_alias.clone(),
                                        self.column_ident(
                                            target_col,
                                            &relationship.target_collection,
                                        )?,
                                    ]))
                                }

                                if self.request.variables.is_some() {
                                    select.push(
                                        Expr::CompoundIdentifier(vec![
                                            Ident::new_quoted("_vars"),
                                            Ident::new_quoted("_varset_id"),
                                        ])
                                        .into_select(Some("_varset_id")),
                                    );
                                    group_by.push(Expr::CompoundIdentifier(vec![
                                        Ident::new_quoted("_vars"),
                                        Ident::new_quoted("_varset_id"),
                                    ]));
                                }

                                let table = self
                                    .collection_ident(&relationship.target_collection)?
                                    .into_table_factor()
                                    .alias(&join_alias);

                                let (table, base_joins) = if self.request.variables.is_some() {
                                    (
                                        ObjectName(vec![Ident::new_quoted("_vars")])
                                            .into_table_factor(),
                                        vec![Join {
                                            relation: table,
                                            join_operator: JoinOperator::CrossJoin,
                                        }],
                                    )
                                } else {
                                    (table, vec![])
                                };

                                let mut join_index = 1;

                                let mut additional_joins = vec![];
                                let mut additional_predicate = vec![];

                                if let Some(expression) = &first_element.predicate {
                                    let (predicate, predicate_joins) = self.filter_expression(
                                        expression,
                                        &join_alias,
                                        &relationship.target_collection,
                                        false,
                                        &mut join_index,
                                    )?;

                                    additional_predicate.push(predicate);

                                    for predicate_join in predicate_joins {
                                        additional_joins.push(predicate_join);
                                    }
                                }

                                let mut last_join_alias = join_alias;
                                let mut last_collection_name = &relationship.target_collection;

                                for path_element in path.iter().skip(1) {
                                    let join_alias =
                                        Ident::new_quoted(format!("_exists_{join_index}"));
                                    join_index += 1;

                                    let relationship =
                                        self.collection_relationship(&path_element.relationship)?;

                                    let join_exprs = relationship
                                        .column_mapping
                                        .iter()
                                        .map(|(source_col, target_col)| {
                                            Ok(Expr::BinaryOp {
                                                left: Expr::CompoundIdentifier(vec![
                                                    last_join_alias.clone(),
                                                    self.column_ident(
                                                        source_col,
                                                        last_collection_name,
                                                    )?,
                                                ])
                                                .into_box(),
                                                op: BinaryOperator::Eq,
                                                right: Expr::CompoundIdentifier(vec![
                                                    join_alias.clone(),
                                                    self.column_ident(
                                                        target_col,
                                                        &relationship.target_collection,
                                                    )?,
                                                ])
                                                .into_box(),
                                            })
                                        })
                                        .collect::<Result<Vec<_>, _>>()?;

                                    let join_operator = join_exprs
                                        .into_iter()
                                        .reduce(and_reducer)
                                        .map(JoinConstraint::On)
                                        .map(JoinOperator::Inner)
                                        .unwrap_or(JoinOperator::CrossJoin);

                                    let relation = self
                                        .collection_ident(&relationship.target_collection)?
                                        .into_table_factor()
                                        .alias(&join_alias);

                                    let join = Join {
                                        relation,
                                        join_operator,
                                    };

                                    additional_joins.push(join);

                                    if let Some(expression) = &path_element.predicate {
                                        let (predicate, predicate_joins) = self.filter_expression(
                                            expression,
                                            &join_alias,
                                            &relationship.target_collection,
                                            false,
                                            &mut join_index,
                                        )?;

                                        additional_predicate.push(predicate);

                                        for predicate_join in predicate_joins {
                                            additional_joins.push(predicate_join);
                                        }
                                    }

                                    last_join_alias = join_alias;
                                    last_collection_name = &relationship.target_collection;
                                }

                                select.push(
                                    Function::new_unquoted("groupArray")
                                        .args(vec![Expr::CompoundIdentifier(vec![
                                            last_join_alias,
                                            self.column_ident(
                                                comparison_column_name,
                                                last_collection_name,
                                            )?,
                                        ])
                                        .into_arg()])
                                        .into_expr()
                                        .into_select(Some("_values")),
                                );

                                let joins =
                                    base_joins.into_iter().chain(additional_joins).collect();

                                let from = vec![table.into_table_with_joins(joins)];

                                let predicate =
                                    additional_predicate.into_iter().reduce(and_reducer);

                                (
                                    Query::new()
                                        .select(select)
                                        .from(from)
                                        .predicate(predicate)
                                        .group_by(group_by),
                                    last_collection_name,
                                )
                            };

                            let mut join_exprs = relationship
                                .column_mapping
                                .iter()
                                .map(|(source_col, target_col)| {
                                    Ok(Expr::BinaryOp {
                                        left: Expr::CompoundIdentifier(vec![
                                            previous_join_alias.clone(),
                                            self.column_ident(source_col, &current_collection)?,
                                        ])
                                        .into_box(),
                                        op: BinaryOperator::Eq,
                                        right: Expr::CompoundIdentifier(vec![
                                            current_join_alias.clone(),
                                            Ident::new_quoted(format!("_relkey_{target_col}")),
                                        ])
                                        .into_box(),
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()?;

                            if self.request.variables.is_some() {
                                join_exprs.push(Expr::BinaryOp {
                                    left: Expr::CompoundIdentifier(vec![
                                        Ident::new_quoted("_vars"),
                                        Ident::new_quoted("_varset_id"),
                                    ])
                                    .into_box(),
                                    op: BinaryOperator::Eq,
                                    right: Expr::CompoundIdentifier(vec![
                                        current_join_alias.clone(),
                                        Ident::new_quoted("_varset_id"),
                                    ])
                                    .into_box(),
                                })
                            }

                            let join_operator = join_exprs
                                .into_iter()
                                .reduce(and_reducer)
                                .map(JoinConstraint::On)
                                .map(JoinOperator::LeftOuter)
                                .unwrap_or(JoinOperator::CrossJoin);

                            let join = Join {
                                relation: subquery
                                    .into_table_factor()
                                    .alias(current_join_alias.clone()),
                                join_operator,
                            };
                            (join, current_join_alias, last_collection_name)
                        };

                        let column_ident = Ident::new_unquoted(format!("_value_{name_index}"));
                        *name_index += 1;

                        let values_ident = Expr::CompoundIdentifier(vec![
                            join_alias,
                            Ident::new_quoted("_values"),
                        ]);

                        Ok(ComparisonColumn::new_grouped(
                            column_ident,
                            join,
                            values_ident,
                            self.column_data_type(comparison_column_name, last_collection_name)?,
                        ))
                    } else {
                        let mut additional_joins = vec![];
                        let mut additional_predicates = vec![];

                        let mut last_join_alias = current_join_alias.clone();
                        let mut last_collection_name = current_collection.clone();

                        for path_element in path {
                            let join_alias = Ident::new_quoted(format!("_exists_{name_index}"));
                            *name_index += 1;

                            let relationship =
                                self.collection_relationship(&path_element.relationship)?;

                            let join_exprs = relationship
                                .column_mapping
                                .iter()
                                .map(|(source_col, target_col)| {
                                    Ok(Expr::BinaryOp {
                                        left: Expr::CompoundIdentifier(vec![
                                            last_join_alias.clone(),
                                            self.column_ident(source_col, last_collection_name)?,
                                        ])
                                        .into_box(),
                                        op: BinaryOperator::Eq,
                                        right: Expr::CompoundIdentifier(vec![
                                            join_alias.clone(),
                                            self.column_ident(
                                                target_col,
                                                &relationship.target_collection,
                                            )?,
                                        ])
                                        .into_box(),
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()?;

                            let join_operator = join_exprs
                                .into_iter()
                                .reduce(and_reducer)
                                .map(JoinConstraint::On)
                                .map(JoinOperator::Inner)
                                .unwrap_or(JoinOperator::CrossJoin);

                            let table = self
                                .collection_ident(&relationship.target_collection)?
                                .into_table_factor()
                                .alias(&join_alias);

                            let join = Join {
                                relation: table,
                                join_operator,
                            };

                            additional_joins.push(join);

                            if let Some(expression) = &path_element.predicate {
                                let (predicate, predicate_joins) = self.filter_expression(
                                    expression,
                                    &join_alias,
                                    &relationship.target_collection,
                                    false,
                                    name_index,
                                )?;

                                additional_predicates.push(predicate);

                                for join in predicate_joins {
                                    additional_joins.push(join)
                                }
                            }

                            last_join_alias = join_alias;
                            last_collection_name = &relationship.target_collection;
                        }

                        let column_ident = Expr::CompoundIdentifier(vec![
                            last_join_alias,
                            self.column_ident(comparison_column_name, last_collection_name)?,
                        ]);

                        Ok(ComparisonColumn::new_flat(
                            column_ident,
                            additional_joins,
                            additional_predicates.into_iter().reduce(and_reducer),
                            self.column_data_type(comparison_column_name, last_collection_name)?,
                        ))
                    }
                } else {
                    let column_ident = Expr::CompoundIdentifier(vec![
                        current_join_alias.clone(),
                        self.column_ident(comparison_column_name, current_collection)?,
                    ]);
                    Ok(ComparisonColumn::new_simple(
                        column_ident,
                        self.column_data_type(comparison_column_name, current_collection)?,
                    ))
                }
            }
            models::ComparisonTarget::RootCollectionColumn { name } => {
                if current_is_origin {
                    let column_ident = Expr::CompoundIdentifier(vec![
                        current_join_alias.clone(),
                        self.column_ident(name, current_collection)?,
                    ]);
                    Ok(ComparisonColumn::new_simple(
                        column_ident,
                        self.column_data_type(name, current_collection)?,
                    ))
                } else {
                    Err(QueryBuilderError::NotSupported(
                        "Comparisons to root not supported".to_string(),
                    ))
                }
            }
        }
    }

    fn collection_relationship(
        &self,
        relationship: &str,
    ) -> Result<&models::Relationship, QueryBuilderError> {
        self.request
            .collection_relationships
            .get(relationship)
            .ok_or(QueryBuilderError::MissingRelationship(
                relationship.to_string(),
            ))
    }
    fn collection_ident(&self, collection_alias: &str) -> Result<ObjectName, QueryBuilderError> {
        // todo: get the collection name based on the alias from config
        let table = self
            .configuration
            .tables
            .iter()
            .find(|t| t.alias == collection_alias)
            .ok_or_else(|| QueryBuilderError::UnknownTable(collection_alias.to_owned()))?;

        Ok(ObjectName(vec![
            Ident::new_quoted(&table.schema),
            Ident::new_quoted(&table.name),
        ]))
    }
    fn column_ident(
        &self,
        column_alias: &str,
        collection_alias: &str,
    ) -> Result<Ident, QueryBuilderError> {
        // todo: get column name based on column alias and collection alias
        let table = self
            .configuration
            .tables
            .iter()
            .find(|t| t.alias == collection_alias)
            .ok_or_else(|| QueryBuilderError::UnknownTable(collection_alias.to_owned()))?;

        let column = table
            .columns
            .iter()
            .find(|c| c.alias == column_alias)
            .ok_or_else(|| {
                QueryBuilderError::UnknownColumn(
                    column_alias.to_owned(),
                    collection_alias.to_owned(),
                )
            })?;

        Ok(Ident::new_quoted(&column.name))
    }
    fn column_data_type(
        &self,
        column_alias: &str,
        collection_alias: &str,
    ) -> Result<String, QueryBuilderError> {
        // todo: get column name based on column alias and collection alias
        let table = self
            .configuration
            .tables
            .iter()
            .find(|t| t.alias == collection_alias)
            .ok_or_else(|| QueryBuilderError::UnknownTable(collection_alias.to_owned()))?;

        let column = table
            .columns
            .iter()
            .find(|c| c.alias == column_alias)
            .ok_or_else(|| {
                QueryBuilderError::UnknownColumn(
                    column_alias.to_owned(),
                    collection_alias.to_owned(),
                )
            })?;

        Ok(column.data_type.to_owned())
    }
}

fn aggregate_function(
    name: &str,
) -> Result<ClickHouseSingleColumnAggregateFunction, QueryBuilderError> {
    ClickHouseSingleColumnAggregateFunction::from_str(name)
        .map_err(|_err| QueryBuilderError::UnknownSingleColumnAggregateFunction(name.to_string()))
}

fn and_reducer(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::And,
        right: Box::new(right),
    }
}

fn or_reducer(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Or,
        right: Box::new(right),
    }
}
