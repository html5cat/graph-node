use graphql_parser::query as q;
use std::error::Error;
use std::fmt;

use graph::prelude::{GraphQLError, Position};

/// GraphQL execution error.
#[derive(Clone, Debug)]
pub enum ExecutionError {
    OperationNameRequired,
    OperationNotFound(String),
    NotSupported(String),
    NoRootQueryObjectType,
    NoRootSubscriptionObjectType,
    ResolveEntityError(Position, String),
    NonNullError(Position, String),
    ListValueError(Position, String),
    NamedTypeError(String),
    AbstractTypeError(String),
    InvalidArgumentError(Position, String, q::Value),
    MissingArgumentError(Position, String),
}

impl GraphQLError for ExecutionError {
    fn locations(&self) -> Vec<Position> {
        match self {
            ExecutionError::ResolveEntityError(pos, _)
            | ExecutionError::NonNullError(pos, _)
            | ExecutionError::ListValueError(pos, _)
            | ExecutionError::InvalidArgumentError(pos, _, _)
            | ExecutionError::MissingArgumentError(pos, _) => vec![pos.clone()],
            _ => vec![],
        }
    }
}

impl Error for ExecutionError {
    fn description(&self) -> &str {
        "Query execution error"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExecutionError::OperationNameRequired => write!(f, "Operation name required"),
            ExecutionError::OperationNotFound(s) => write!(f, "Operation name not found: {}", s),
            ExecutionError::NotSupported(s) => write!(f, "Not supported: {}", s),
            ExecutionError::NoRootQueryObjectType => {
                write!(f, "No root Query type defined in the schema")
            }
            ExecutionError::NoRootSubscriptionObjectType => {
                write!(f, "No root Subscription type defined in the schema")
            }
            ExecutionError::ResolveEntityError(_, s) => {
                write!(f, "Failed to resolve entity: {}", s)
            }
            ExecutionError::NonNullError(_, s) => {
                write!(f, "Null value resolved for non-null field: {}", s)
            }
            ExecutionError::ListValueError(_, s) => {
                write!(f, "Non-list value resolved for list field: {}", s)
            }
            ExecutionError::NamedTypeError(s) => write!(f, "Failed to resolve named type: {}", s),
            ExecutionError::AbstractTypeError(s) => {
                write!(f, "Failed to resolve abstract type: {}", s)
            }
            ExecutionError::InvalidArgumentError(_, s, v) => {
                write!(f, "Invalid value provided for argument \"{}\": {:?}", s, v)
            }
            ExecutionError::MissingArgumentError(_, s) => {
                write!(f, "No value provided for required argument: {}", s)
            }
        }
    }
}
