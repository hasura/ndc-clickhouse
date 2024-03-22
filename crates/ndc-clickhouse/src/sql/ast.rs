#![allow(dead_code)]
use std::fmt;

mod parameter_extractor;
use parameter_extractor::ParameterExtractor;

//.A statement containing placeholders where parameters used to be
// Should be paired with the corresponding parameters for execution
#[derive(Debug, Clone)]
pub struct ParameterizedStatement(Statement);

impl ParameterizedStatement {
    pub fn format<S: Into<String>>(self, format: S) -> Self {
        Self(self.0.format(format))
    }
    pub fn explain(self) -> Self {
        Self(self.0.explain())
    }
    pub fn to_parameterized_sql_string(&self) -> String {
        self.0.to_string()
    }
}

/// A statement that contains parameters that may be user generated
/// Vulnerable to SQL injection
/// Should never be sent to the database, but may be useful as user-facing output when explaining requests
#[derive(Debug, Clone)]
pub struct UnsafeInlinedStatement(Statement);

impl UnsafeInlinedStatement {
    pub fn format<S: Into<String>>(self, format: S) -> Self {
        Self(self.0.format(format))
    }
    pub fn explain(self) -> Self {
        Self(self.0.explain())
    }
    pub fn to_unsafe_sql_string(&self) -> String {
        self.0.to_string()
    }
    pub fn into_parameterized_statement(self) -> (ParameterizedStatement, Vec<(String, String)>) {
        ParameterExtractor::extract_statement_parameters(self)
    }
}

#[derive(Debug, Clone)]
pub struct Statement {
    query: Query,
    format: Option<String>,
    explain: bool,
}

impl Statement {
    pub fn format<S: Into<String>>(self, format: S) -> Self {
        Self {
            query: self.query,
            format: Some(format.into()),
            explain: self.explain,
        }
    }
    pub fn explain(self) -> Self {
        Self {
            explain: true,
            ..self
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Statement {
            query,
            format,
            explain,
        } = self;

        if *explain {
            write!(f, "EXPLAIN ")?;
        }

        write!(f, "{}", query)?;

        if let Some(format) = &format {
            write!(f, " FORMAT {}", format)?;
        }

        write!(f, ";")
    }
}

#[derive(Debug, Default, Clone)]
pub struct Query {
    with: Vec<WithItem>,
    select: Vec<SelectItem>,
    from: Vec<TableWithJoins>,
    predicate: Option<Expr>,
    group_by: Vec<Expr>,
    order_by: Vec<OrderByExpr>,
    limit_by: Option<LimitByExpr>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl Query {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with(self, with: Vec<WithItem>) -> Self {
        Self { with, ..self }
    }
    pub fn select(self, select: Vec<SelectItem>) -> Self {
        Self { select, ..self }
    }
    pub fn from(self, from: Vec<TableWithJoins>) -> Self {
        Self { from, ..self }
    }
    pub fn predicate(self, predicate: Option<Expr>) -> Self {
        Self { predicate, ..self }
    }
    pub fn group_by(self, group_by: Vec<Expr>) -> Self {
        Self { group_by, ..self }
    }
    pub fn order_by(self, order_by: Vec<OrderByExpr>) -> Self {
        Self { order_by, ..self }
    }
    pub fn limit_by(self, limit_by: Option<LimitByExpr>) -> Self {
        Self { limit_by, ..self }
    }
    pub fn limit(self, limit: Option<u64>) -> Self {
        Self { limit, ..self }
    }
    pub fn offset(self, offset: Option<u64>) -> Self {
        Self { offset, ..self }
    }
    pub fn into_statement(self) -> UnsafeInlinedStatement {
        UnsafeInlinedStatement(Statement {
            query: self,
            format: None,
            explain: false,
        })
    }
    pub fn into_table_factor(self) -> TableFactor {
        TableFactor::Derived {
            subquery: Box::new(self),
            alias: None,
        }
    }
    pub fn into_with_item<S: Into<String>>(self, alias: S) -> WithItem {
        WithItem::Query {
            query: Box::new(self),
            alias: Ident::new_quoted(alias),
        }
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.with.is_empty() {
            write!(f, "WITH {} ", display_separated(&self.with, ", "))?;
        }
        write!(f, "SELECT {}", display_separated(&self.select, ", "))?;
        if !self.from.is_empty() {
            write!(f, " FROM {}", display_separated(&self.from, ", "))?;
        }
        if let Some(predicate) = &self.predicate {
            write!(f, " WHERE {}", predicate)?;
        }
        if !self.group_by.is_empty() {
            write!(f, " GROUP BY {}", display_separated(&self.group_by, ", "))?;
        }
        if !self.order_by.is_empty() {
            write!(f, " ORDER BY {}", display_separated(&self.order_by, ", "))?;
        }
        if let Some(limit_by) = &self.limit_by {
            write!(f, " LIMIT {}", limit_by.limit)?;
            if let Some(offset) = limit_by.offset {
                write!(f, " OFFSET {}", offset)?;
            }
            write!(f, " BY {}", display_separated(&limit_by.by, ", "))?;
        }
        if let Some(limit) = &self.limit {
            write!(f, " LIMIT {}", limit)?;
        }
        if let Some(offset) = &self.offset {
            write!(f, " OFFSET {}", offset)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum WithItem {
    Expr { expr: Expr, alias: Ident },
    Query { query: Box<Query>, alias: Ident },
}

impl fmt::Display for WithItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WithItem::Expr { expr, alias } => write!(f, "{expr} AS {alias}"),
            WithItem::Query { query, alias } => write!(f, "{alias} AS ({query})"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LimitByExpr {
    limit: u64,
    offset: Option<u64>,
    by: Vec<Expr>,
}

impl LimitByExpr {
    pub fn new(limit: Option<u64>, offset: Option<u64>, by: Vec<Expr>) -> LimitByExpr {
        LimitByExpr {
            limit: limit.unwrap_or(u64::MAX),
            offset,
            by,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: Expr,
    pub asc: Option<bool>,
    pub nulls_first: Option<bool>,
}

impl fmt::Display for OrderByExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.expr)?;
        match self.asc {
            Some(true) => write!(f, " ASC")?,
            Some(false) => write!(f, " DESC")?,
            None => (),
        }
        match self.nulls_first {
            Some(true) => write!(f, " NULLS FIRST")?,
            Some(false) => write!(f, " NULLS LAST")?,
            None => (),
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum SelectItem {
    UnnamedExpr(Expr),
    ExprWithAlias { expr: Expr, alias: Ident },
    QualifiedWildcard(ObjectName),
    Wildcard,
}

impl SelectItem {
    pub fn unnamed(expr: Expr) -> Self {
        SelectItem::UnnamedExpr(expr)
    }
    pub fn with_alias<S: Into<Ident>>(expr: Expr, alias: S) -> SelectItem {
        SelectItem::ExprWithAlias {
            expr,
            alias: alias.into(),
        }
    }
}

impl fmt::Display for SelectItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectItem::UnnamedExpr(expr) => write!(f, "{}", expr),
            SelectItem::ExprWithAlias { expr, alias } => write!(f, "{} AS {}", expr, alias),
            SelectItem::QualifiedWildcard(name) => write!(f, "{}.*", name),
            SelectItem::Wildcard => write!(f, "*"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Join {
    pub relation: TableFactor,
    pub join_operator: JoinOperator,
}

impl fmt::Display for Join {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn prefix(constraint: &JoinConstraint) -> &'static str {
            match constraint {
                JoinConstraint::Natural => "NATURAL ",
                _ => "",
            }
        }
        fn suffix(constraint: &'_ JoinConstraint) -> impl fmt::Display + '_ {
            struct Suffix<'a>(&'a JoinConstraint);
            impl<'a> fmt::Display for Suffix<'a> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    match self.0 {
                        JoinConstraint::On(expr) => write!(f, " ON {expr}"),
                        JoinConstraint::Using(attrs) => {
                            write!(f, " USING({})", display_separated(attrs, ", "))
                        }
                        _ => Ok(()),
                    }
                }
            }
            Suffix(constraint)
        }
        match &self.join_operator {
            JoinOperator::Inner(constraint) => write!(
                f,
                " {}JOIN {}{}",
                prefix(constraint),
                self.relation,
                suffix(constraint)
            ),
            JoinOperator::LeftOuter(constraint) => write!(
                f,
                " {}LEFT JOIN {}{}",
                prefix(constraint),
                self.relation,
                suffix(constraint)
            ),
            JoinOperator::RightOuter(constraint) => write!(
                f,
                " {}RIGHT JOIN {}{}",
                prefix(constraint),
                self.relation,
                suffix(constraint)
            ),
            JoinOperator::FullOuter(constraint) => write!(
                f,
                " {}FULL JOIN {}{}",
                prefix(constraint),
                self.relation,
                suffix(constraint)
            ),
            JoinOperator::CrossJoin => write!(f, " CROSS JOIN {}", self.relation),
        }
    }
}

#[derive(Debug, Clone)]
pub enum JoinOperator {
    Inner(JoinConstraint),
    LeftOuter(JoinConstraint),
    RightOuter(JoinConstraint),
    FullOuter(JoinConstraint),
    CrossJoin,
}

#[derive(Debug, Clone)]
pub enum JoinConstraint {
    On(Expr),
    Using(Vec<Ident>),
    Natural,
    None,
}

#[derive(Debug, Clone)]
pub struct TableWithJoins {
    relation: TableFactor,
    joins: Vec<Join>,
}

impl fmt::Display for TableWithJoins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.relation)?;
        for join in &self.joins {
            write!(f, " {}", join)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum TableFactor {
    Table {
        name: ObjectName,
        alias: Option<Ident>,
    },
    Derived {
        subquery: Box<Query>,
        alias: Option<Ident>,
    },
    TableFunction {
        function: Function,
        alias: Option<Ident>,
    },
}

impl TableFactor {
    pub fn alias<S: Into<Ident>>(self, alias: S) -> Self {
        let alias = Some(alias.into());
        match self {
            TableFactor::Table { name, alias: _ } => TableFactor::Table { name, alias },
            TableFactor::Derived { subquery, alias: _ } => TableFactor::Derived { subquery, alias },
            TableFactor::TableFunction { function, alias: _ } => {
                TableFactor::TableFunction { function, alias }
            }
        }
    }
    pub fn into_table_with_joins(self, joins: Vec<Join>) -> TableWithJoins {
        TableWithJoins {
            relation: self,
            joins,
        }
    }
}

impl fmt::Display for TableFactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TableFactor::Table { name, alias } => {
                write!(f, "{}", name)?;
                if let Some(alias) = alias {
                    write!(f, " AS {}", alias)?;
                }
            }
            TableFactor::Derived { subquery, alias } => {
                write!(f, "({})", subquery)?;
                if let Some(alias) = alias {
                    write!(f, " AS {}", alias)?;
                }
            }
            TableFactor::TableFunction { function, alias } => {
                write!(f, "{}", function)?;
                if let Some(alias) = alias {
                    write!(f, " AS {}", alias)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ObjectName(pub Vec<Ident>);

impl ObjectName {
    pub fn into_table_factor(self) -> TableFactor {
        TableFactor::Table {
            name: self,
            alias: None,
        }
    }
}

impl fmt::Display for ObjectName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", display_separated(&self.0, "."))
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Identifier(Ident),
    CompoundIdentifier(Vec<Ident>),
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    Not(Box<Expr>),
    Nested(Box<Expr>),
    Value(Value),
    Parameter(Parameter),
    Function(Function),
    Lambda(Lambda),
    List(Vec<Expr>),
}

impl Expr {
    pub fn into_select<S: Into<Ident>>(self, alias: Option<S>) -> SelectItem {
        match alias {
            Some(alias) => SelectItem::ExprWithAlias {
                expr: self,
                alias: alias.into(),
            },
            None => SelectItem::UnnamedExpr(self),
        }
    }
    pub fn into_arg(self) -> FunctionArgExpr {
        FunctionArgExpr::Expr(self)
    }
    pub fn into_with_item<S: Into<String>>(self, alias: S) -> WithItem {
        WithItem::Expr {
            expr: self,
            alias: Ident::new_quoted(alias),
        }
    }
    pub fn into_nested(self) -> Expr {
        Expr::Nested(Box::new(self))
    }
    pub fn into_box(self) -> Box<Expr> {
        Box::new(self)
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Identifier(ident) => write!(f, "{}", ident),
            Expr::CompoundIdentifier(idents) => write!(f, "{}", display_separated(idents, ".")),
            Expr::BinaryOp { left, op, right } => write!(f, "{} {} {}", left, op, right),
            Expr::Not(expr) => write!(f, "NOT {expr}"),
            Expr::Nested(expr) => write!(f, "({})", expr),
            Expr::Value(value) => write!(f, "{}", value),
            Expr::Parameter(p) => match p {
                Parameter::Value {
                    value,
                    data_type: _,
                } => write!(f, "{value}"),
                Parameter::Placeholder { name, data_type } => {
                    write!(f, "{{{}:{}}}", name, data_type)
                }
            },
            Expr::Function(function) => write!(f, "{}", function),
            Expr::Lambda(lambda) => write!(f, "{}", lambda),
            Expr::List(list) => write!(f, "({})", display_separated(list, ", ")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Parameter {
    Value { data_type: String, value: Value },
    Placeholder { data_type: String, name: String },
}

impl Parameter {
    pub fn new(value: Value, data_type: String) -> Self {
        Self::Value { data_type, value }
    }
    pub fn into_expr(self) -> Expr {
        Expr::Parameter(self)
    }
}

#[derive(Debug, Clone)]
pub struct Lambda {
    args: Vec<Ident>,
    expr: Box<Expr>,
}

impl Lambda {
    pub fn new(args: Vec<Ident>, expr: Expr) -> Self {
        Self {
            args,
            expr: Box::new(expr),
        }
    }
    pub fn into_expr(self) -> Expr {
        Expr::Lambda(self)
    }
}

impl fmt::Display for Lambda {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}) -> {}",
            display_separated(&self.args, ", "),
            self.expr
        )
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: ObjectName,
    pub args: Vec<FunctionArgExpr>,
    pub over: Option<WindowSpec>,
    pub distinct: bool,
}

impl Function {
    pub fn new_quoted<N: Into<String>>(name: N) -> Self {
        Function {
            name: ObjectName(vec![Ident::new_quoted(name)]),
            args: vec![],
            over: None,
            distinct: false,
        }
    }
    pub fn new_unquoted<N: Into<String>>(name: N) -> Self {
        Function {
            name: ObjectName(vec![Ident::new_unquoted(name)]),
            args: vec![],
            over: None,
            distinct: false,
        }
    }
    pub fn args(self, args: Vec<FunctionArgExpr>) -> Self {
        Self { args, ..self }
    }
    pub fn over(self, over: Option<WindowSpec>) -> Self {
        Self { over, ..self }
    }
    pub fn distinct(self, distinct: bool) -> Self {
        Self { distinct, ..self }
    }
    pub fn into_expr(self) -> Expr {
        Expr::Function(self)
    }
    pub fn into_table_factor(self) -> TableFactor {
        TableFactor::TableFunction {
            function: self,
            alias: None,
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({}{})",
            self.name,
            if self.distinct { "DISTINCT " } else { "" },
            display_separated(&self.args, ", ")
        )?;
        if let Some(over) = &self.over {
            write!(f, " OVER ({})", over)?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct WindowSpec {
    pub partition_by: Vec<Expr>,
    pub order_by: Vec<OrderByExpr>,
}

impl fmt::Display for WindowSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.partition_by.is_empty() {
            write!(
                f,
                "PARTITION BY {}",
                display_separated(&self.partition_by, ", ")
            )?;
        }
        if !self.order_by.is_empty() {
            if !self.partition_by.is_empty() {
                write!(f, " ")?;
            }
            write!(f, "ORDER BY {}", display_separated(&self.order_by, ", "))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum FunctionArgExpr {
    Expr(Expr),
    /// Qualified wildcard, e.g. `alias.*` or `schema.table.*`.
    QualifiedWildcard(ObjectName),
    /// An unqualified `*`
    Wildcard,
}

impl fmt::Display for FunctionArgExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionArgExpr::Expr(expr) => write!(f, "{}", expr),
            FunctionArgExpr::QualifiedWildcard(name) => write!(f, "{}.*", name),
            FunctionArgExpr::Wildcard => write!(f, "*"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Not,
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOperator::Not => write!(f, "NOT"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Gt,
    Lt,
    GtEq,
    LtEq,
    Eq,
    NotEq,
    Like,
    NotLike,
    ILike,
    NotILike,
    And,
    Or,
    In,
    NotIn,
    Is,
    IsNot,
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOperator::Gt => write!(f, ">"),
            BinaryOperator::Lt => write!(f, "<"),
            BinaryOperator::GtEq => write!(f, ">="),
            BinaryOperator::LtEq => write!(f, "<="),
            BinaryOperator::Eq => write!(f, "="),
            BinaryOperator::NotEq => write!(f, "!="),
            BinaryOperator::Like => write!(f, "LIKE"),
            BinaryOperator::NotLike => write!(f, "NOT LIKE"),
            BinaryOperator::ILike => write!(f, "ILIKE"),
            BinaryOperator::NotILike => write!(f, "NOT ILIKE"),
            BinaryOperator::And => write!(f, "AND"),
            BinaryOperator::Or => write!(f, "OR"),
            BinaryOperator::In => write!(f, "IN"),
            BinaryOperator::NotIn => write!(f, "NOT IN"),
            BinaryOperator::Is => write!(f, "IS"),
            BinaryOperator::IsNot => write!(f, "IS NOT"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(String),
    SingleQuotedString(String),
    Boolean(bool),
    Null,
}

impl Value {
    pub fn into_expr(self) -> Expr {
        Expr::Value(self)
    }
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Boolean(b),
            serde_json::Value::Number(n) => Value::Number(n.to_string()),
            serde_json::Value::String(s) => Value::SingleQuotedString(s),
            // note we may need to convert complex types into escaped json strings rather than json strings.
            serde_json::Value::Array(_) => Value::SingleQuotedString(value.to_string()),
            serde_json::Value::Object(_) => Value::SingleQuotedString(value.to_string()),
        }
    }
}

impl From<&serde_json::Value> for Value {
    fn from(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Boolean(b.to_owned()),
            serde_json::Value::Number(n) => Value::Number(n.to_string()),
            serde_json::Value::String(s) => Value::SingleQuotedString(s.to_owned()),
            serde_json::Value::Array(_) => Value::SingleQuotedString(value.to_string()),
            serde_json::Value::Object(_) => Value::SingleQuotedString(value.to_string()),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::SingleQuotedString(s) => {
                // clickhouse docs state that backslash and single quotes must be escaped
                // docs: https://clickhouse.com/docs/en/sql-reference/syntax#syntax-string-literal
                let escaped_value = s.to_owned().replace('\\', r#"\\"#).replace('\'', r#"\'"#);
                write!(f, "'{}'", escaped_value)
            }
            Value::Boolean(b) => {
                if *b {
                    write!(f, "TRUE")
                } else {
                    write!(f, "FALSE")
                }
            }
            Value::Null => write!(f, "NULL"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ident {
    value: String,
    quoted: bool,
}

impl Ident {
    pub fn new<S: Into<String>>(value: S, quoted: bool) -> Self {
        Self {
            value: value.into(),
            quoted,
        }
    }

    pub fn new_quoted<S: Into<String>>(value: S) -> Self {
        Self {
            value: value.into(),
            quoted: true,
        }
    }
    pub fn new_unquoted<S: Into<String>>(value: S) -> Self {
        Self {
            value: value.into(),
            quoted: false,
        }
    }
    pub fn into_expr(self) -> Expr {
        Expr::Identifier(self)
    }
}

impl From<&str> for Ident {
    fn from(value: &str) -> Self {
        Ident::new_quoted(value)
    }
}
impl From<String> for Ident {
    fn from(value: String) -> Self {
        Ident::new_quoted(value)
    }
}
impl From<&String> for Ident {
    fn from(value: &String) -> Self {
        Ident::new_quoted(value)
    }
}
impl From<&Ident> for Ident {
    fn from(value: &Ident) -> Self {
        value.clone()
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.quoted {
            write!(f, "\"{}\"", self.value)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

pub struct DisplaySeparated<'a, T>
where
    T: fmt::Display,
{
    slice: &'a [T],
    separator: &'static str,
}

fn display_separated<'a, T>(slice: &'a [T], separator: &'static str) -> DisplaySeparated<'a, T>
where
    T: fmt::Display,
{
    DisplaySeparated { slice, separator }
}

impl<'a, T> fmt::Display for DisplaySeparated<'a, T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for t in self.slice {
            if first {
                first = false;
            } else {
                write!(f, "{}", self.separator)?;
            }
            write!(f, "{}", t)?;
        }
        Ok(())
    }
}
