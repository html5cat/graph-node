extern crate graph;
extern crate graphql_parser;
extern crate indexmap;
extern crate inflector;
#[macro_use]
extern crate serde_derive;
extern crate serde;

/// Utilities for working with GraphQL schemas.
pub mod schema;

/// Utilities for schema introspection.
pub mod introspection;

/// Utilities for executing GraphQL.
mod execution;

/// Utilities for executing GraphQL queries and working with query ASTs.
pub mod query;

/// Utilities for executing GraphQL subscriptions.
mod subscription;

/// Utilities for working with GraphQL values.
mod values;

/// Utilities for querying `Store` components.
mod store;

/// Prelude that exports the most important traits and types.
pub mod prelude {
    pub use super::execution::{Execution, ExecutionError, Resolver};
    pub use super::introspection::{introspection_schema, IntrospectionResolver};
    pub use super::query::{execute_query, Query, QueryOptions, QueryResult};
    pub use super::schema::{api_schema, APISchemaError};
    pub use super::store::{build_query, StoreResolver};
    pub use super::subscription::execute_subscription;
    pub use super::values::{object_value, MaybeCoercible, SerializableValue};
}
