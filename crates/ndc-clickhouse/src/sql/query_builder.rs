mod collection_context;
mod comparison_column;
mod error;
mod typecasting;

use self::{collection_context::CollectionContext, typecasting::RowsetTypeString};
use super::ast::*;
use crate::schema::{
    ClickHouseBinaryComparisonOperator, ClickHouseSingleColumnAggregateFunction,
    ClickHouseTypeDefinition,
};
use common::{
    clickhouse_parser::{
        datatype::ClickHouseDataType, parameterized_query::ParameterizedQueryElement,
    },
    config::ServerConfig,
};
use comparison_column::ComparisonColumn;
pub use error::QueryBuilderError;
use indexmap::IndexMap;
use ndc_sdk::models;
use std::{collections::BTreeMap, iter, str::FromStr};

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
    fn root_query(&self) -> Result<UnsafeInlinedStatement, QueryBuilderError> {
        let collection = CollectionContext::new(&self.request.collection, &self.request.arguments);
        let query = &self.request.query;

        let select = vec![Function::new_unquoted("toJSONString")
            .args(vec![Function::new_unquoted("groupArray")
                .args(vec![Function::new_unquoted("cast")
                    .args(vec![
                        Expr::CompoundIdentifier(vec![
                            Ident::new_quoted("_rowset"),
                            Ident::new_quoted("_rowset"),
                        ])
                        .into_arg(),
                        Expr::Value(Value::SingleQuotedString(
                            RowsetTypeString::new(
                                collection.alias(),
                                query,
                                &self.request.collection_relationships,
                                self.configuration,
                            )?
                            .into_cast_type()
                            .to_string(),
                        ))
                        .into_arg(),
                    ])
                    .into_expr()
                    .into_arg()])
                .into_expr()
                .into_arg()])
            .into_expr()
            .into_select(Some("rowsets"))];

        let with = if let Some(variables) = &self.request.variables {
            // we make the following assumptions here:
            // variables is an array with at least one item
            // all items in variables are object with keys and primitive values
            // all items have the same keys
            // if those assumptions aren't true, we may generate a misshapen object
            // clickhouse will reject this at runtime
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
                    ClickHouseDataType::String.into(),
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

        let rowset_subquery = self
            .rowset_subquery(&collection, &vec![], query)?
            .into_table_factor()
            .alias("_rowset");

        let from = if self.request.variables.is_some() {
            let table = ObjectName(vec![Ident::new_quoted("_vars")])
                .into_table_factor()
                .alias("_vars");

            let join_expr = Expr::BinaryOp {
                left: Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_vars"),
                    Ident::new_quoted("_varset_id"),
                ])
                .into_box(),
                op: BinaryOperator::Eq,
                right: Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_rowset".to_owned()),
                    Ident::new_quoted("_varset_id"),
                ])
                .into_box(),
            };

            let join = Join {
                relation: rowset_subquery,
                join_operator: JoinOperator::LeftOuter(JoinConstraint::On(join_expr)),
            };

            vec![table.into_table_with_joins(vec![join])]
        } else {
            vec![rowset_subquery.into_table_with_joins(vec![])]
        };

        let order_by = if self.request.variables.is_some() {
            vec![OrderByExpr {
                expr: Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_vars"),
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
            .format("TabSeparatedRaw"))
    }
    fn rowset_subquery(
        &self,
        current_collection: &CollectionContext,
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
                                .args(vec![FunctionArgExpr::Wildcard.into_arg()])
                                .into_expr(),
                            models::Aggregate::ColumnCount {
                                distinct,
                                column: _,
                                field_path: _,
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
                                field_path: _,
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
                    .collect::<Result<Vec<_>, QueryBuilderError>>()?;
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
        current_collection: &CollectionContext,
        relkeys: &Vec<&String>,
        query: &models::Query,
    ) -> Result<Query, QueryBuilderError> {
        let (table, mut base_joins) = if self.request.variables.is_some() {
            let table = ObjectName(vec![Ident::new_quoted("_vars")])
                .into_table_factor()
                .alias("_vars");

            let joins = vec![Join {
                relation: self.collection_ident(current_collection)?.alias("_origin"),
                join_operator: JoinOperator::CrossJoin,
            }];
            (table, joins)
        } else {
            let table = self.collection_ident(current_collection)?.alias("_origin");
            (table, vec![])
        };

        let mut select = vec![];

        if let Some(fields) = &query.fields {
            let mut rel_index = 0;
            for (alias, field) in fields {
                match field {
                    models::Field::Column {
                        column,
                        fields,
                        arguments: _,
                    } => {
                        let data_type = self.column_data_type(column, current_collection)?;
                        let column_definition = ClickHouseTypeDefinition::from_table_column(
                            &data_type,
                            column,
                            current_collection.alias(),
                            &self.configuration.namespace_separator,
                        );

                        let column_ident =
                            vec![Ident::new_quoted("_origin"), self.column_ident(column)];

                        if let Some((expr, mut joins)) = self.column_accessor(
                            column_ident,
                            &column_definition,
                            false,
                            fields.as_ref(),
                            &mut rel_index,
                        )? {
                            select.push(expr.into_select(Some(format!("_field_{alias}"))));
                            base_joins.append(&mut joins);
                        } else {
                            let expr = Expr::CompoundIdentifier(vec![
                                Ident::new_quoted("_origin"),
                                self.column_ident(column),
                            ]);
                            select.push(expr.into_select(Some(format!("_field_{alias}"))));
                        }
                    }
                    models::Field::Relationship {
                        query,
                        relationship,
                        arguments,
                    } => {
                        let (expr, join) = self.field_relationship(
                            alias,
                            &mut rel_index,
                            &[Ident::new_quoted("_origin")],
                            query,
                            relationship,
                            arguments,
                        )?;

                        select.push(expr.into_select(Some(format!("_field_{alias}"))));
                        base_joins.push(join);
                    }
                }
            }
        }

        if let Some(aggregates) = &query.aggregates {
            for (alias, aggregate) in aggregates {
                if let models::Aggregate::ColumnCount { column, .. }
                | models::Aggregate::SingleColumn { column, .. } = aggregate
                {
                    let expr = Expr::CompoundIdentifier(vec![
                        Ident::new_quoted("_origin"),
                        self.column_ident(column),
                    ]);
                    select.push(expr.into_select(Some(format!("_agg_{alias}"))))
                }
            }
        }

        for relkey in relkeys {
            select.push(
                Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_origin"),
                    self.column_ident(relkey),
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

        let (order_by_exprs, order_by_joins) = self.order_by(&query.order_by)?;

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
                    self.column_ident(relkey),
                ]))
            })
            .collect::<Result<Vec<_>, QueryBuilderError>>()?;

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
    fn order_by(
        &self,
        order_by: &Option<models::OrderBy>,
    ) -> Result<(Vec<OrderByExpr>, Vec<Join>), QueryBuilderError> {
        let mut order_by_exprs = vec![];
        let mut order_by_joins = vec![];

        if let Some(order_by) = &order_by {
            let mut order_by_index = 0;

            for element in &order_by.elements {
                match &element.target {
                    models::OrderByTarget::Column {
                        name,
                        path,
                        field_path: _,
                    } if path.is_empty() => {
                        let expr = Expr::CompoundIdentifier(vec![
                            Ident::new_quoted("_origin"),
                            self.column_ident(name),
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

                        if let (
                            models::RelationshipType::Array,
                            models::OrderByTarget::Column { .. },
                        ) = (&relationship.relationship_type, &element.target)
                        {
                            return Err(QueryBuilderError::NotSupported(
                                "order by column across array relationship".to_string(),
                            ));
                        }
                        let relationship_collection = CollectionContext::from_relationship(
                            relationship,
                            &first_element.arguments,
                        );

                        let subquery = {
                            let mut select = vec![];
                            let mut group_by = vec![];
                            let mut limit_by = vec![];

                            let join_alias = Ident::new_quoted("_order_by_0");

                            for target_col in relationship.column_mapping.values() {
                                select.push(
                                    Expr::CompoundIdentifier(vec![
                                        join_alias.clone(),
                                        self.column_ident(target_col),
                                    ])
                                    .into_select(Some(format!("_relkey_{target_col}"))),
                                );
                                group_by.push(Expr::CompoundIdentifier(vec![
                                    join_alias.clone(),
                                    self.column_ident(target_col),
                                ]));
                                limit_by.push(Expr::CompoundIdentifier(vec![
                                    join_alias.clone(),
                                    self.column_ident(target_col),
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
                                .collection_ident(&relationship_collection)?
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
                                    &relationship_collection,
                                    false,
                                    &mut join_index,
                                )?;

                                additional_predicate.push(predicate);

                                for predicate_join in predicate_joins {
                                    additional_joins.push(predicate_join);
                                }
                            }

                            let mut last_join_alias = join_alias;

                            for path_element in path.iter().skip(1) {
                                let join_alias =
                                    Ident::new_quoted(format!("_order_by_{join_index}"));
                                join_index += 1;

                                let relationship =
                                    self.collection_relationship(&path_element.relationship)?;

                                if let (
                                    models::RelationshipType::Array,
                                    models::OrderByTarget::Column { .. },
                                ) = (&relationship.relationship_type, &element.target)
                                {
                                    return Err(QueryBuilderError::NotSupported(
                                        "order by column across array relationship".to_string(),
                                    ));
                                }

                                let relationship_collection = CollectionContext::from_relationship(
                                    relationship,
                                    &path_element.arguments,
                                );

                                let join_exprs = relationship
                                    .column_mapping
                                    .iter()
                                    .map(|(source_col, target_col)| {
                                        Ok(Expr::BinaryOp {
                                            left: Expr::CompoundIdentifier(vec![
                                                last_join_alias.clone(),
                                                self.column_ident(source_col),
                                            ])
                                            .into_box(),
                                            op: BinaryOperator::Eq,
                                            right: Expr::CompoundIdentifier(vec![
                                                join_alias.clone(),
                                                self.column_ident(target_col),
                                            ])
                                            .into_box(),
                                        })
                                    })
                                    .collect::<Result<Vec<_>, QueryBuilderError>>()?;

                                let join_operator = join_exprs
                                    .into_iter()
                                    .reduce(and_reducer)
                                    .map(JoinConstraint::On)
                                    .map(JoinOperator::Inner)
                                    .unwrap_or(JoinOperator::CrossJoin);

                                let relation = self
                                    .collection_ident(&relationship_collection)?
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
                                        &relationship_collection,
                                        false,
                                        &mut join_index,
                                    )?;

                                    additional_predicate.push(predicate);

                                    for predicate_join in predicate_joins {
                                        additional_joins.push(predicate_join);
                                    }
                                }

                                last_join_alias = join_alias;
                            }

                            match &element.target {
                                models::OrderByTarget::Column {
                                    name,
                                    path: _,
                                    field_path: _,
                                } => {
                                    let column = Expr::CompoundIdentifier(vec![
                                        last_join_alias,
                                        self.column_ident(name),
                                    ]);
                                    group_by.push(column.clone());
                                    select.push(column.into_select(Some("_order_by_value")));
                                }
                                models::OrderByTarget::SingleColumnAggregate {
                                    column,
                                    function,
                                    path: _,
                                    field_path: _,
                                } => {
                                    let column = Expr::CompoundIdentifier(vec![
                                        last_join_alias,
                                        self.column_ident(column),
                                    ]);
                                    select.push(
                                        aggregate_function(function)?
                                            .as_expr(column)
                                            .into_select(Some("_order_by_value")),
                                    );
                                }
                                models::OrderByTarget::StarCountAggregate { path: _ } => {
                                    let count = Function::new_unquoted("COUNT")
                                        .args(vec![FunctionArgExpr::Wildcard.into_arg()]);
                                    select.push(
                                        count.into_expr().into_select(Some("_order_by_value")),
                                    );
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
                                            self.column_ident(source_col),
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
                                .collect::<Result<Vec<_>, QueryBuilderError>>()?;

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

        Ok((order_by_exprs, order_by_joins))
    }
    fn field_relationship(
        &self,
        field_alias: &str,
        name_index: &mut u32,
        target_path: &[Ident],
        query: &models::Query,
        relationship: &str,
        arguments: &BTreeMap<String, models::RelationshipArgument>,
    ) -> Result<(Expr, Join), QueryBuilderError> {
        let join_alias = format!("_rel_{name_index}_{field_alias}");
        *name_index += 1;

        let relationship = self.collection_relationship(relationship)?;
        let relationship_collection = CollectionContext::from_relationship(relationship, arguments);

        let mut join_expr = relationship
            .column_mapping
            .iter()
            .map(|(source_col, target_col)| {
                Ok(Expr::BinaryOp {
                    left: Expr::CompoundIdentifier(
                        target_path
                            .iter()
                            .cloned()
                            .chain(iter::once(Ident::new_quoted(source_col)))
                            .collect(),
                    )
                    .into_box(),
                    op: BinaryOperator::Eq,
                    right: Expr::CompoundIdentifier(vec![
                        Ident::new_quoted(&join_alias),
                        Ident::new_quoted(format!("_relkey_{target_col}")),
                    ])
                    .into_box(),
                })
            })
            .collect::<Result<Vec<_>, QueryBuilderError>>()?;

        if self.request.variables.is_some() {
            join_expr.push(Expr::BinaryOp {
                left: Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_vars"),
                    Ident::new_quoted("_varset_id"),
                ])
                .into_box(),
                op: BinaryOperator::Eq,
                right: Expr::CompoundIdentifier(vec![
                    Ident::new_quoted(&join_alias),
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
                .rowset_subquery(&relationship_collection, &relkeys, query)?
                .into_table_factor()
                .alias(&join_alias),
            join_operator,
        };

        let expr = Expr::CompoundIdentifier(vec![
            Ident::new_quoted(&join_alias),
            Ident::new_quoted("_rowset"),
        ]);

        Ok((expr, join))
    }
    fn filter_expression(
        &self,
        expression: &models::Expression,
        current_join_alias: &Ident,
        current_collection: &CollectionContext,
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
                        ClickHouseDataType::Array(Box::new(left_col.data_type()))
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
                        Parameter::new(value.into(), right_col_type.clone().into()).into_expr(),
                        right_col_type,
                    ),

                    models::ComparisonValue::Variable { name } => {
                        let column_ident = Expr::CompoundIdentifier(vec![
                            Ident::new_quoted("_vars"),
                            Ident::new_quoted(format!("_var_{name}")),
                        ]);
                        let column_ident = if is_uuid(&right_col_type) {
                            Function::new_unquoted("toUUID")
                                .args(vec![column_ident.into_arg()])
                                .into_expr()
                        } else {
                            column_ident
                        };
                        ComparisonColumn::new_simple(column_ident, right_col_type)
                    }
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
                name_index,
            ),
        }
    }
    fn filter_exists_expression(
        &self,
        in_collection: &models::ExistsInCollection,
        expression: &Option<Box<models::Expression>>,
        previous_join_alias: &Ident,
        name_index: &mut u32,
    ) -> Result<(Expr, Vec<Join>), QueryBuilderError> {
        let exists_join_ident = Ident::new_quoted(format!("_exists_{}", name_index));
        *name_index += 1;

        let join_subquery = {
            let target_collection = match in_collection {
                models::ExistsInCollection::Related {
                    relationship,
                    arguments,
                } => {
                    let relationship = self.collection_relationship(relationship)?;
                    CollectionContext::from_relationship(relationship, arguments)
                }
                models::ExistsInCollection::Unrelated {
                    collection,
                    arguments,
                } => CollectionContext::new_unrelated(collection, arguments),
            };

            let subquery_origin_alias = Ident::new_quoted(format!("_exists_{}", name_index));
            *name_index += 1;

            let (predicate, predicate_joins) = match expression {
                Some(expression) => {
                    let (predicate, predicate_joins) = self.filter_expression(
                        expression,
                        &subquery_origin_alias,
                        &target_collection,
                        false,
                        name_index,
                    )?;
                    (Some(predicate), predicate_joins)
                }
                None => (None, vec![]),
            };

            let table = self
                .collection_ident(&target_collection)?
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
                            self.column_ident(target_col),
                        ])
                        .into_select(Some(format!("_relkey_{target_col}"))),
                    );
                    limit_by.push(Expr::CompoundIdentifier(vec![
                        subquery_origin_alias.clone(),
                        self.column_ident(target_col),
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
                            self.column_ident(source_col),
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
                    .collect::<Result<_, QueryBuilderError>>()?
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
        current_collection: &CollectionContext,
        current_is_origin: bool,
        name_index: &mut u32,
    ) -> Result<ComparisonColumn, QueryBuilderError> {
        match column {
            models::ComparisonTarget::Column {
                name: comparison_column_name,
                path,
                field_path: _,
            } => {
                if let Some(first_element) = path.first() {
                    if current_is_origin {
                        let (join, join_alias, last_collection_context) = {
                            let previous_join_alias = current_join_alias.clone();
                            let current_join_alias =
                                Ident::new_quoted(format!("_exists_{name_index}"));
                            *name_index += 1;

                            let relationship =
                                self.collection_relationship(&first_element.relationship)?;
                            let relationship_collection = CollectionContext::from_relationship(
                                relationship,
                                &first_element.arguments,
                            );

                            let (subquery, last_collection_name) = {
                                let mut select = vec![];
                                let mut group_by = vec![];

                                let join_alias = Ident::new_quoted("_exists_0");

                                for target_col in relationship.column_mapping.values() {
                                    select.push(
                                        Expr::CompoundIdentifier(vec![
                                            join_alias.clone(),
                                            self.column_ident(target_col),
                                        ])
                                        .into_select(Some(format!("_relkey_{target_col}"))),
                                    );
                                    group_by.push(Expr::CompoundIdentifier(vec![
                                        join_alias.clone(),
                                        self.column_ident(target_col),
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
                                    .collection_ident(&relationship_collection)?
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
                                        &relationship_collection,
                                        false,
                                        &mut join_index,
                                    )?;

                                    additional_predicate.push(predicate);

                                    for predicate_join in predicate_joins {
                                        additional_joins.push(predicate_join);
                                    }
                                }

                                let mut last_join_alias = join_alias;
                                let mut last_collection_context = relationship_collection;

                                for path_element in path.iter().skip(1) {
                                    let join_alias =
                                        Ident::new_quoted(format!("_exists_{join_index}"));
                                    join_index += 1;

                                    let relationship =
                                        self.collection_relationship(&path_element.relationship)?;
                                    let relationship_collection =
                                        CollectionContext::from_relationship(
                                            relationship,
                                            &path_element.arguments,
                                        );

                                    let join_exprs = relationship
                                        .column_mapping
                                        .iter()
                                        .map(|(source_col, target_col)| {
                                            Ok(Expr::BinaryOp {
                                                left: Expr::CompoundIdentifier(vec![
                                                    last_join_alias.clone(),
                                                    self.column_ident(source_col),
                                                ])
                                                .into_box(),
                                                op: BinaryOperator::Eq,
                                                right: Expr::CompoundIdentifier(vec![
                                                    join_alias.clone(),
                                                    self.column_ident(target_col),
                                                ])
                                                .into_box(),
                                            })
                                        })
                                        .collect::<Result<Vec<_>, QueryBuilderError>>()?;

                                    let join_operator = join_exprs
                                        .into_iter()
                                        .reduce(and_reducer)
                                        .map(JoinConstraint::On)
                                        .map(JoinOperator::Inner)
                                        .unwrap_or(JoinOperator::CrossJoin);

                                    let relation = self
                                        .collection_ident(&relationship_collection)?
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
                                            &relationship_collection,
                                            false,
                                            &mut join_index,
                                        )?;

                                        additional_predicate.push(predicate);

                                        for predicate_join in predicate_joins {
                                            additional_joins.push(predicate_join);
                                        }
                                    }

                                    last_join_alias = join_alias;
                                    last_collection_context = relationship_collection;
                                }

                                select.push(
                                    Function::new_unquoted("groupArray")
                                        .args(vec![Expr::CompoundIdentifier(vec![
                                            last_join_alias,
                                            self.column_ident(comparison_column_name),
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
                                    last_collection_context,
                                )
                            };

                            let mut join_exprs = relationship
                                .column_mapping
                                .iter()
                                .map(|(source_col, target_col)| {
                                    Ok(Expr::BinaryOp {
                                        left: Expr::CompoundIdentifier(vec![
                                            previous_join_alias.clone(),
                                            self.column_ident(source_col),
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
                                .collect::<Result<Vec<_>, QueryBuilderError>>()?;

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
                            self.column_data_type(
                                comparison_column_name,
                                &last_collection_context,
                            )?,
                        ))
                    } else {
                        let mut additional_joins = vec![];
                        let mut additional_predicates = vec![];

                        let mut last_join_alias = current_join_alias.clone();
                        let mut last_collection_context: CollectionContext =
                            current_collection.to_owned();

                        for path_element in path {
                            let join_alias = Ident::new_quoted(format!("_exists_{name_index}"));
                            *name_index += 1;

                            let relationship =
                                self.collection_relationship(&path_element.relationship)?;
                            let relationship_collection = CollectionContext::from_relationship(
                                relationship,
                                &path_element.arguments,
                            );

                            let join_exprs = relationship
                                .column_mapping
                                .iter()
                                .map(|(source_col, target_col)| {
                                    Ok(Expr::BinaryOp {
                                        left: Expr::CompoundIdentifier(vec![
                                            last_join_alias.clone(),
                                            self.column_ident(source_col),
                                        ])
                                        .into_box(),
                                        op: BinaryOperator::Eq,
                                        right: Expr::CompoundIdentifier(vec![
                                            join_alias.clone(),
                                            self.column_ident(target_col),
                                        ])
                                        .into_box(),
                                    })
                                })
                                .collect::<Result<Vec<_>, QueryBuilderError>>()?;

                            let join_operator = join_exprs
                                .into_iter()
                                .reduce(and_reducer)
                                .map(JoinConstraint::On)
                                .map(JoinOperator::Inner)
                                .unwrap_or(JoinOperator::CrossJoin);

                            let table = self
                                .collection_ident(&relationship_collection)?
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
                                    &relationship_collection,
                                    false,
                                    name_index,
                                )?;

                                additional_predicates.push(predicate);

                                for join in predicate_joins {
                                    additional_joins.push(join)
                                }
                            }

                            last_join_alias = join_alias;
                            last_collection_context = relationship_collection;
                        }

                        let column_ident = Expr::CompoundIdentifier(vec![
                            last_join_alias,
                            self.column_ident(comparison_column_name),
                        ]);

                        Ok(ComparisonColumn::new_flat(
                            column_ident,
                            additional_joins,
                            additional_predicates.into_iter().reduce(and_reducer),
                            self.column_data_type(
                                comparison_column_name,
                                &last_collection_context,
                            )?,
                        ))
                    }
                } else {
                    let column_ident = Expr::CompoundIdentifier(vec![
                        current_join_alias.clone(),
                        self.column_ident(comparison_column_name),
                    ]);
                    Ok(ComparisonColumn::new_simple(
                        column_ident,
                        self.column_data_type(comparison_column_name, current_collection)?,
                    ))
                }
            }
            models::ComparisonTarget::RootCollectionColumn {
                name,
                field_path: _,
            } => {
                if current_is_origin {
                    let column_ident = Expr::CompoundIdentifier(vec![
                        current_join_alias.clone(),
                        self.column_ident(name),
                    ]);
                    Ok(ComparisonColumn::new_simple(
                        column_ident,
                        self.column_data_type(name, current_collection)?,
                    ))
                } else {
                    Err(QueryBuilderError::NotSupported(
                        "Comparisons to root".to_string(),
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
    fn collection_ident(
        &self,
        collection: &CollectionContext,
    ) -> Result<TableFactor, QueryBuilderError> {
        if let Some(table) = self.configuration.tables.get(collection.alias()) {
            let table_argument_type = |argument_name: &str| {
                table.arguments.get(argument_name).ok_or_else(|| {
                    QueryBuilderError::UnknownTableArgument {
                        table: collection.alias().to_owned(),
                        argument: argument_name.to_owned(),
                    }
                })
            };
            let variable_argument = |arg_name: &String, variable_name: &String| {
                let argument_type = table_argument_type(arg_name)?;
                let column_ident = Expr::CompoundIdentifier(vec![
                    Ident::new_quoted("_vars"),
                    Ident::new_quoted(format!("_var_{variable_name}")),
                ]);

                let column_ident = if is_uuid(argument_type) {
                    Function::new_unquoted("toUUID")
                        .args(vec![column_ident.into_arg()])
                        .into_expr()
                } else {
                    column_ident
                };
                Ok(column_ident.into_arg().name(Ident::new_quoted(arg_name)))
            };
            let literal_argument = |arg_name: &String, value: &serde_json::Value| {
                Ok(Expr::Parameter(Parameter::new(
                    value.into(),
                    table_argument_type(arg_name)?.to_owned().into(),
                ))
                .into_arg()
                .name(Ident::new_quoted(arg_name)))
            };
            let table_name = ObjectName(vec![
                Ident::new_quoted(&table.schema),
                Ident::new_quoted(&table.name),
            ]);
            if collection.has_arguments() {
                let arguments = match collection {
                    CollectionContext::Base {
                        collection_alias: _,
                        arguments,
                    } => arguments
                        .iter()
                        .map(|(arg_name, arg)| match arg {
                            models::Argument::Variable { name } => {
                                variable_argument(arg_name, name)
                            }
                            models::Argument::Literal { value } => {
                                literal_argument(arg_name, value)
                            }
                        })
                        .collect::<Result<Vec<FunctionArg>, _>>()?,
                    CollectionContext::Relationship {
                        collection_alias: _,
                        arguments,
                        relationship_arguments,
                    } => relationship_arguments
                        .iter()
                        .chain(arguments.iter())
                        .map(|(arg_name, arg)| match arg {
                            models::RelationshipArgument::Variable { name } => {
                                variable_argument(arg_name, name)
                            }
                            models::RelationshipArgument::Literal { value } => {
                                literal_argument(arg_name, value)
                            }
                            models::RelationshipArgument::Column { .. } => {
                                Err(QueryBuilderError::NotSupported(
                                    "Column argument value".to_string(),
                                ))
                            }
                        })
                        .collect::<Result<Vec<FunctionArg>, _>>()?,
                    CollectionContext::UnrelatedRelationship {
                        collection_alias: _,
                        arguments,
                    } => arguments
                        .iter()
                        .map(|(arg_name, arg)| match arg {
                            models::RelationshipArgument::Variable { name } => {
                                variable_argument(arg_name, name)
                            }
                            models::RelationshipArgument::Literal { value } => {
                                literal_argument(arg_name, value)
                            }
                            models::RelationshipArgument::Column { .. } => {
                                Err(QueryBuilderError::NotSupported(
                                    "Column argument value".to_string(),
                                ))
                            }
                        })
                        .collect::<Result<Vec<FunctionArg>, _>>()?,
                };

                let table_function = table_name.into_table_function().args(arguments);

                Ok(table_function.into_table_factor())
            } else {
                Ok(table_name.into_table_factor())
            }
        } else if let Some(query) = self.configuration.queries.get(collection.alias()) {
            let get_argument = |name| match collection {
                CollectionContext::Base {
                    collection_alias: _,
                    arguments,
                } => arguments.get(name).map(|arg| match arg {
                    models::Argument::Variable { .. } => Err(QueryBuilderError::NotSupported(
                        "native query variable argument".to_string(),
                    )),
                    models::Argument::Literal { value } => Ok(value),
                }),
                CollectionContext::Relationship {
                    collection_alias: _,
                    arguments,
                    relationship_arguments,
                } => arguments
                    .get(name)
                    .or_else(|| relationship_arguments.get(name))
                    .map(|arg| match arg {
                        models::RelationshipArgument::Variable { .. } => {
                            Err(QueryBuilderError::NotSupported(
                                "native query variable argument".to_string(),
                            ))
                        }
                        models::RelationshipArgument::Literal { value } => Ok(value),
                        models::RelationshipArgument::Column { .. } => {
                            Err(QueryBuilderError::NotSupported(
                                "native query column argument".to_string(),
                            ))
                        }
                    }),
                CollectionContext::UnrelatedRelationship {
                    collection_alias: _,
                    arguments,
                } => arguments.get(name).map(|arg| match arg {
                    models::RelationshipArgument::Variable { .. } => {
                        Err(QueryBuilderError::NotSupported(
                            "native query variable argument".to_string(),
                        ))
                    }
                    models::RelationshipArgument::Literal { value } => Ok(value),
                    models::RelationshipArgument::Column { .. } => Err(
                        QueryBuilderError::NotSupported("native query column argument".to_string()),
                    ),
                }),
            };

            let elements = query
                .query
                .elements
                .iter()
                .map(|element| match element {
                    ParameterizedQueryElement::String(s) => {
                        Ok(NativeQueryElement::String(s.to_owned()))
                    }
                    ParameterizedQueryElement::Parameter(p) => get_argument(p.name.value())
                        .transpose()?
                        .map(|value| {
                            NativeQueryElement::Parameter(Parameter::new(
                                value.into(),
                                p.r#type.clone(),
                            ))
                        })
                        .ok_or_else(|| QueryBuilderError::MissingNativeQueryArgument {
                            query: collection.alias().to_owned(),
                            argument: p.name.value().to_owned(),
                        }),
                })
                .collect::<Result<_, _>>()?;

            Ok(NativeQuery::new(elements).into_table_factor())
        } else {
            Err(QueryBuilderError::UnknownTable(
                collection.alias().to_owned(),
            ))
        }
    }
    fn column_ident(&self, column_alias: &str) -> Ident {
        Ident::new_quoted(column_alias)
    }
    fn column_data_type(
        &self,
        column_alias: &str,
        collection: &CollectionContext,
    ) -> Result<ClickHouseDataType, QueryBuilderError> {
        let return_type = self
            .configuration
            .tables
            .get(collection.alias())
            .map(|table| &table.return_type)
            .or_else(|| {
                self.configuration
                    .queries
                    .get(collection.alias())
                    .map(|query| &query.return_type)
            })
            .ok_or_else(|| QueryBuilderError::UnknownTable(collection.alias().to_owned()))?;

        let table_type = &self
            .configuration
            .table_types
            .get(return_type)
            .ok_or_else(|| QueryBuilderError::UnknownTableType(return_type.to_owned()))?;

        let column_type = table_type.columns.get(column_alias).ok_or_else(|| {
            QueryBuilderError::UnknownColumn(column_alias.to_owned(), collection.alias().to_owned())
        })?;

        Ok(column_type.to_owned())
    }
    fn column_accessor(
        &self,
        column_ident: Vec<Ident>,
        type_definition: &ClickHouseTypeDefinition,
        traversed_array: bool,
        field_selector: Option<&models::NestedField>,
        rel_index: &mut u32,
    ) -> Result<Option<(Expr, Vec<Join>)>, QueryBuilderError> {
        if let Some(fields) = field_selector {
            match fields {
                models::NestedField::Array(inner) => {
                    let element_type = match type_definition.non_nullable() {
                        ClickHouseTypeDefinition::Array { element_type } => &**element_type,
                        _ => {
                            return Err(QueryBuilderError::ColumnTypeMismatch {
                                expected: "Array".to_string(),
                                got: type_definition.cast_type().to_string(),
                            });
                        }
                    };

                    let ident = Ident::new_unquoted("_value");

                    if let Some((expr, joins)) = self.column_accessor(
                        vec![ident.clone()],
                        element_type,
                        true,
                        Some(&inner.fields),
                        rel_index,
                    )? {
                        if !joins.is_empty() {
                            return Err(QueryBuilderError::Unexpected("column accessor should not return relationship joins after array traversal".to_string()));
                        }

                        let expr = Function::new_unquoted("arrayMap")
                            .args(vec![
                                Lambda::new(vec![ident], expr).into_expr().into_arg(),
                                Expr::CompoundIdentifier(column_ident).into_arg(),
                            ])
                            .into_expr();

                        Ok(Some((expr, joins)))
                    } else {
                        Ok(None)
                    }
                }
                models::NestedField::Object(inner) => {
                    let object_type_fields = match type_definition.non_nullable() {
                        ClickHouseTypeDefinition::Object { name: _, fields } => fields,
                        _ => {
                            return Err(QueryBuilderError::ColumnTypeMismatch {
                                expected: "Tuple/Object".to_string(),
                                got: type_definition.cast_type().to_string(),
                            });
                        }
                    };

                    let chain_ident = |i: &str| -> Vec<Ident> {
                        column_ident
                            .clone()
                            .into_iter()
                            .chain(iter::once(Ident::new_quoted(i)))
                            .collect()
                    };

                    let mut required_joins = vec![];
                    let mut column_accessors = vec![];
                    let mut required_columns = vec![];
                    let mut accessor_required = false;
                    for (alias, field) in &inner.fields {
                        match field {
                            models::Field::Column {
                                column,
                                fields,
                                arguments: _,
                            } => {
                                required_columns.push(column);

                                let type_definition =
                                    object_type_fields.get(column).ok_or_else(|| {
                                        QueryBuilderError::UnknownSubField {
                                            field_name: column.to_owned(),
                                            data_type: type_definition.cast_type().to_string(),
                                        }
                                    })?;

                                let column_ident = chain_ident(column);
                                if let Some((expr, mut joins)) = self.column_accessor(
                                    column_ident.clone(),
                                    type_definition,
                                    traversed_array,
                                    fields.as_ref(),
                                    rel_index,
                                )? {
                                    accessor_required = true;
                                    required_joins.append(&mut joins);
                                    column_accessors.push(expr);
                                } else {
                                    column_accessors.push(Expr::CompoundIdentifier(column_ident));
                                }
                            }
                            models::Field::Relationship {
                                query,
                                relationship,
                                arguments,
                            } => {
                                if traversed_array {
                                    return Err(QueryBuilderError::NotSupported(
                                        "Relationships with fields nested in arrays".to_string(),
                                    ));
                                }

                                let (expr, join) = self.field_relationship(
                                    alias,
                                    rel_index,
                                    &column_ident,
                                    query,
                                    relationship,
                                    arguments,
                                )?;

                                required_joins.push(join);
                                column_accessors.push(expr);
                            }
                        }
                    }

                    let all_columns: Vec<_> = object_type_fields.keys().collect();
                    if !accessor_required
                        && required_joins.is_empty()
                        && required_columns == all_columns
                    {
                        Ok(None)
                    } else {
                        let expr = Function::new_unquoted("tuple")
                            .args(
                                column_accessors
                                    .into_iter()
                                    .map(|expr| expr.into_arg())
                                    .collect(),
                            )
                            .into_expr();

                        Ok(Some((expr, required_joins)))
                    }
                }
            }
        } else {
            Ok(None)
        }
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

fn is_uuid(t: &ClickHouseDataType) -> bool {
    match t {
        ClickHouseDataType::Nullable(t) => is_uuid(t),
        ClickHouseDataType::Uuid => true,
        _ => false,
    }
}
