use bon::bon;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operator {
    And,
    Or,
    Not,
}

impl Operator {
    pub fn as_str(&self) -> &'static str {
        match self {
            Operator::And => "and",
            Operator::Or => "or",
            Operator::Not => "not",
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Comparator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contain,
    Like,
    In,
    Nin,
}

impl Comparator {
    pub fn as_str(&self) -> &'static str {
        match self {
            Comparator::Eq => "eq",
            Comparator::Ne => "ne",
            Comparator::Gt => "gt",
            Comparator::Gte => "gte",
            Comparator::Lt => "lt",
            Comparator::Lte => "lte",
            Comparator::Contain => "contain",
            Comparator::Like => "like",
            Comparator::In => "in",
            Comparator::Nin => "nin",
        }
    }
}

impl fmt::Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorOrComparator {
    Operator(Operator),
    Comparator(Comparator),
}

impl From<Operator> for OperatorOrComparator {
    fn from(op: Operator) -> Self {
        OperatorOrComparator::Operator(op)
    }
}

impl From<Comparator> for OperatorOrComparator {
    fn from(comp: Comparator) -> Self {
        OperatorOrComparator::Comparator(comp)
    }
}

impl fmt::Display for OperatorOrComparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperatorOrComparator::Operator(op) => write!(f, "{}", op),
            OperatorOrComparator::Comparator(comp) => write!(f, "{}", comp),
        }
    }
}

pub trait Visitor {
    type Output;

    fn allowed_comparators(&self) -> Option<&[Comparator]> {
        None
    }

    fn allowed_operators(&self) -> Option<&[Operator]> {
        None
    }

    fn validate_func(&self, func: OperatorOrComparator) -> Result<()> {
        match func {
            OperatorOrComparator::Operator(op) => {
                if let Some(allowed) = self.allowed_operators()
                    && !allowed.contains(&op)
                {
                    return Err(Error::Other(format!(
                        "Received disallowed operator {}. Allowed operators are {:?}",
                        op, allowed
                    )));
                }
            }
            OperatorOrComparator::Comparator(comp) => {
                if let Some(allowed) = self.allowed_comparators()
                    && !allowed.contains(&comp)
                {
                    return Err(Error::Other(format!(
                        "Received disallowed comparator {}. Allowed comparators are {:?}",
                        comp, allowed
                    )));
                }
            }
        }
        Ok(())
    }

    fn visit_operation(&self, operation: &Operation) -> Result<Self::Output>;

    fn visit_comparison(&self, comparison: &Comparison) -> Result<Self::Output>;

    fn visit_structured_query(&self, structured_query: &StructuredQuery) -> Result<Self::Output>;
}

pub trait Expr: fmt::Debug {
    fn expr_name(&self) -> &'static str;

    fn accept<V: Visitor>(&self, visitor: &V) -> Result<V::Output>;
}

pub trait FilterDirective: Expr {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub comparator: Comparator,
    pub attribute: String,
    pub value: serde_json::Value,
}

#[bon]
impl Comparison {
    #[builder]
    pub fn new(
        comparator: Comparator,
        attribute: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        Comparison {
            comparator,
            attribute: attribute.into(),
            value: value.into(),
        }
    }
}

impl Expr for Comparison {
    fn expr_name(&self) -> &'static str {
        "comparison"
    }

    fn accept<V: Visitor>(&self, visitor: &V) -> Result<V::Output> {
        visitor.visit_comparison(self)
    }
}

impl FilterDirective for Comparison {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub operator: Operator,
    pub arguments: Vec<FilterDirectiveEnum>,
}

#[bon]
impl Operation {
    #[builder]
    pub fn new(operator: Operator, arguments: Vec<FilterDirectiveEnum>) -> Self {
        Operation {
            operator,
            arguments,
        }
    }

    pub fn and(arguments: Vec<FilterDirectiveEnum>) -> Self {
        Self::builder().operator(Operator::And).arguments(arguments).build()
    }

    pub fn or(arguments: Vec<FilterDirectiveEnum>) -> Self {
        Self::builder().operator(Operator::Or).arguments(arguments).build()
    }

    pub fn not(argument: FilterDirectiveEnum) -> Self {
        Self::builder().operator(Operator::Not).arguments(vec![argument]).build()
    }
}

impl Expr for Operation {
    fn expr_name(&self) -> &'static str {
        "operation"
    }

    fn accept<V: Visitor>(&self, visitor: &V) -> Result<V::Output> {
        visitor.visit_operation(self)
    }
}

impl FilterDirective for Operation {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilterDirectiveEnum {
    Comparison(Comparison),
    Operation(Operation),
}

impl FilterDirectiveEnum {
    pub fn accept<V: Visitor>(&self, visitor: &V) -> Result<V::Output> {
        match self {
            FilterDirectiveEnum::Comparison(c) => visitor.visit_comparison(c),
            FilterDirectiveEnum::Operation(o) => visitor.visit_operation(o),
        }
    }
}

impl From<Comparison> for FilterDirectiveEnum {
    fn from(comparison: Comparison) -> Self {
        FilterDirectiveEnum::Comparison(comparison)
    }
}

impl From<Operation> for FilterDirectiveEnum {
    fn from(operation: Operation) -> Self {
        FilterDirectiveEnum::Operation(operation)
    }
}

impl Expr for FilterDirectiveEnum {
    fn expr_name(&self) -> &'static str {
        match self {
            FilterDirectiveEnum::Comparison(_) => "comparison",
            FilterDirectiveEnum::Operation(_) => "operation",
        }
    }

    fn accept<V: Visitor>(&self, visitor: &V) -> Result<V::Output> {
        match self {
            FilterDirectiveEnum::Comparison(c) => visitor.visit_comparison(c),
            FilterDirectiveEnum::Operation(o) => visitor.visit_operation(o),
        }
    }
}

impl FilterDirective for FilterDirectiveEnum {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredQuery {
    pub query: String,
    pub filter: Option<FilterDirectiveEnum>,
    pub limit: Option<usize>,
}

#[bon]
impl StructuredQuery {
    #[builder]
    pub fn new(
        query: impl Into<String>,
        filter: Option<FilterDirectiveEnum>,
        limit: Option<usize>,
    ) -> Self {
        StructuredQuery {
            query: query.into(),
            filter,
            limit,
        }
    }

    pub fn query_only(query: impl Into<String>) -> Self {
        Self::builder().query(query).build()
    }

    pub fn with_filter(query: impl Into<String>, filter: impl Into<FilterDirectiveEnum>) -> Self {
        Self::builder().query(query).filter(filter.into()).build()
    }
}

impl Expr for StructuredQuery {
    fn expr_name(&self) -> &'static str {
        "structured_query"
    }

    fn accept<V: Visitor>(&self, visitor: &V) -> Result<V::Output> {
        visitor.visit_structured_query(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_snake_case(name: &str) -> String {
        let mut snake_case = String::new();
        for (i, char) in name.chars().enumerate() {
            if char.is_uppercase() && i != 0 {
                snake_case.push('_');
                snake_case.push(char.to_ascii_lowercase());
            } else {
                snake_case.push(char.to_ascii_lowercase());
            }
        }
        snake_case
    }

    #[test]
    fn test_operator_display() {
        assert_eq!(Operator::And.to_string(), "and");
        assert_eq!(Operator::Or.to_string(), "or");
        assert_eq!(Operator::Not.to_string(), "not");
    }

    #[test]
    fn test_comparator_display() {
        assert_eq!(Comparator::Eq.to_string(), "eq");
        assert_eq!(Comparator::Ne.to_string(), "ne");
        assert_eq!(Comparator::Gt.to_string(), "gt");
        assert_eq!(Comparator::Gte.to_string(), "gte");
        assert_eq!(Comparator::Lt.to_string(), "lt");
        assert_eq!(Comparator::Lte.to_string(), "lte");
        assert_eq!(Comparator::Contain.to_string(), "contain");
        assert_eq!(Comparator::Like.to_string(), "like");
        assert_eq!(Comparator::In.to_string(), "in");
        assert_eq!(Comparator::Nin.to_string(), "nin");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Comparison"), "comparison");
        assert_eq!(to_snake_case("Operation"), "operation");
        assert_eq!(to_snake_case("StructuredQuery"), "structured_query");
        assert_eq!(to_snake_case("FilterDirective"), "filter_directive");
    }

    #[test]
    fn test_comparison_creation() {
        let comparison = Comparison::new(Comparator::Eq, "field", "value");
        assert_eq!(comparison.comparator, Comparator::Eq);
        assert_eq!(comparison.attribute, "field");
        assert_eq!(comparison.value, serde_json::json!("value"));
    }

    #[test]
    fn test_operation_creation() {
        let comparison = Comparison::new(Comparator::Gt, "age", 18);
        let operation = Operation::and(vec![comparison.into()]);
        assert_eq!(operation.operator, Operator::And);
        assert_eq!(operation.arguments.len(), 1);
    }

    #[test]
    fn test_structured_query_creation() {
        let filter = Comparison::new(Comparator::Eq, "status", "active");
        let query = StructuredQuery::with_filter("search term", filter);
        assert_eq!(query.query, "search term");
        assert!(query.filter.is_some());
        assert!(query.limit.is_none());
    }

    struct TestVisitor {
        allowed_operators: Vec<Operator>,
        allowed_comparators: Vec<Comparator>,
    }

    impl TestVisitor {
        fn new() -> Self {
            TestVisitor {
                allowed_operators: vec![Operator::And, Operator::Or],
                allowed_comparators: vec![Comparator::Eq, Comparator::Ne],
            }
        }
    }

    impl Visitor for TestVisitor {
        type Output = String;

        fn allowed_operators(&self) -> Option<&[Operator]> {
            Some(&self.allowed_operators)
        }

        fn allowed_comparators(&self) -> Option<&[Comparator]> {
            Some(&self.allowed_comparators)
        }

        fn visit_operation(&self, operation: &Operation) -> Result<Self::Output> {
            self.validate_func(operation.operator.into())?;
            Ok(format!("operation:{}", operation.operator))
        }

        fn visit_comparison(&self, comparison: &Comparison) -> Result<Self::Output> {
            self.validate_func(comparison.comparator.into())?;
            Ok(format!(
                "comparison:{}:{}",
                comparison.attribute, comparison.comparator
            ))
        }

        fn visit_structured_query(
            &self,
            structured_query: &StructuredQuery,
        ) -> Result<Self::Output> {
            Ok(format!("query:{}", structured_query.query))
        }
    }

    #[test]
    fn test_visitor_validation() {
        let visitor = TestVisitor::new();

        assert!(visitor.validate_func(Operator::And.into()).is_ok());
        assert!(visitor.validate_func(Operator::Or.into()).is_ok());

        assert!(visitor.validate_func(Operator::Not.into()).is_err());

        assert!(visitor.validate_func(Comparator::Eq.into()).is_ok());
        assert!(visitor.validate_func(Comparator::Ne.into()).is_ok());

        assert!(visitor.validate_func(Comparator::Gt.into()).is_err());
    }

    #[test]
    fn test_visitor_accept() {
        let visitor = TestVisitor::new();

        let comparison = Comparison::new(Comparator::Eq, "field", "value");
        let result = comparison.accept(&visitor).unwrap();
        assert_eq!(result, "comparison:field:eq");

        let operation = Operation::and(vec![comparison.clone().into()]);
        let result = operation.accept(&visitor).unwrap();
        assert_eq!(result, "operation:and");
    }

    #[test]
    fn test_serialization() {
        let comparison = Comparison::new(Comparator::Eq, "field", "value");
        let json = serde_json::to_string(&comparison).unwrap();
        let deserialized: Comparison = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.comparator, comparison.comparator);
        assert_eq!(deserialized.attribute, comparison.attribute);

        let operation = Operation::and(vec![comparison.into()]);
        let json = serde_json::to_string(&operation).unwrap();
        let deserialized: Operation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.operator, operation.operator);
    }
}
