use super::*;

pub struct ParameterExtractor {
    parameter_index: usize,
    parameters: Vec<(String, String)>,
}

impl ParameterExtractor {
    pub fn extract_statement_parameters(
        statement: UnsafeInlinedStatement,
    ) -> (ParameterizedStatement, Vec<(String, String)>) {
        let mut visitor = Self::new();
        let mut statement = statement.0;
        visitor.visit_statement(&mut statement);
        (ParameterizedStatement(statement), visitor.parameters)
    }
    fn new() -> Self {
        Self {
            parameter_index: 0,
            parameters: vec![],
        }
    }
    fn visit_statement(&mut self, statment: &mut Statement) {
        self.visit_query(&mut statment.query)
    }
    fn visit_query(&mut self, query: &mut Query) {
        for with_item in query.with.iter_mut() {
            self.visit_with(with_item)
        }
        for select in query.select.iter_mut() {
            self.visit_select(select)
        }
        for from in query.from.iter_mut() {
            self.visit_from(from)
        }
        if let Some(ref mut predicate) = &mut query.predicate {
            self.visit_expr(predicate)
        }
        for group_by in query.group_by.iter_mut() {
            self.visit_expr(group_by)
        }
        for order_by in query.order_by.iter_mut() {
            self.visit_expr(&mut order_by.expr)
        }
        if let Some(ref mut limit_by) = &mut query.limit_by {
            self.visit_limit_by(limit_by)
        }
    }
    fn visit_with(&mut self, with: &mut WithItem) {
        match with {
            WithItem::Expr { expr, alias: _ } => self.visit_expr(expr),
            WithItem::Query { query, alias: _ } => self.visit_query(query),
        }
    }
    fn visit_select(&mut self, select: &mut SelectItem) {
        match select {
            SelectItem::UnnamedExpr(expr) => self.visit_expr(expr),
            SelectItem::ExprWithAlias { expr, alias: _ } => self.visit_expr(expr),
            SelectItem::QualifiedWildcard(_) => {}
            SelectItem::Wildcard => {}
        }
    }
    fn visit_limit_by(&mut self, limit_by: &mut LimitByExpr) {
        for expr in limit_by.by.iter_mut() {
            self.visit_expr(expr)
        }
    }
    fn visit_expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Identifier(_) => {}
            Expr::CompoundIdentifier(_) => {}
            Expr::BinaryOp { left, op: _, right } => {
                self.visit_expr(left);
                self.visit_expr(right);
            },
            Expr::Not(expr) => self.visit_expr(expr),
            Expr::Nested(expr) => self.visit_expr(expr),
            Expr::Value(_) => {}
            Expr::Parameter(parameter) => self.visit_parameter(parameter),
            Expr::Function(function) => self.visit_function(function),
            Expr::Lambda(lambda) => self.visit_expr(&mut lambda.expr),
            Expr::List(list) => {
                for expr in list.iter_mut() {
                    self.visit_expr(expr)
                }
            }
        }
    }
    fn visit_from(&mut self, from: &mut TableWithJoins) {
        self.visit_relation(&mut from.relation);
        for join in from.joins.iter_mut() {
            self.visit_join(join)
        }
    }
    fn visit_relation(&mut self, relation: &mut TableFactor) {
        match relation {
            TableFactor::Table { .. } => {}
            TableFactor::Derived {
                ref mut subquery,
                alias: _,
            } => self.visit_query(subquery),
            TableFactor::TableFunction {
                ref mut function,
                alias: _,
            } => self.visit_function(function),
        }
    }
    fn visit_join(&mut self, join: &mut Join) {
        self.visit_relation(&mut join.relation);
        match &mut join.join_operator {
            JoinOperator::CrossJoin => {}
            JoinOperator::Inner(constraint)
            | JoinOperator::LeftOuter(constraint)
            | JoinOperator::RightOuter(constraint)
            | JoinOperator::FullOuter(constraint) => match constraint {
                JoinConstraint::On(expr) => self.visit_expr(expr),
                JoinConstraint::Using(_) => {}
                JoinConstraint::Natural => {}
                JoinConstraint::None => {}
            },
        }
    }
    fn visit_function(&mut self, function: &mut Function) {
        for arg in function.args.iter_mut() {
            match arg {
                FunctionArgExpr::Expr(expr) => self.visit_expr(expr),
                FunctionArgExpr::QualifiedWildcard(_) => {}
                FunctionArgExpr::Wildcard => {}
            }
        }
        if let Some(ref mut over) = &mut function.over {
            for partion_by in over.partition_by.iter_mut() {
                self.visit_expr(partion_by)
            }
            for order_by in over.order_by.iter_mut() {
                self.visit_expr(&mut order_by.expr)
            }
        }
    }
    fn visit_parameter(&mut self, parameter: &mut Parameter) {
        match parameter {
            Parameter::Placeholder { .. } => panic!("Attempted to extract parameter that had already been replaced with a placeholder. This is likely a bug"),
            Parameter::Value { data_type, value } => {
                // for single quoted string, we want the underlying string,
                // not the escaped, quoted version we get by calling to_string()
                let value = match value {
                    Value::SingleQuotedString(s) => s.to_owned(),
                    _ => value.to_string()
                };
                self.parameters.push((
                    format!("param_p{}", self.parameter_index),
                    value.to_string()
                ));
                *parameter = Parameter::Placeholder { data_type: data_type.to_owned(), name: format!("p{}", self.parameter_index) };
                self.parameter_index += 1;
            },
        }
    }
}
