use graphql_parser::{query as q, schema as s};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use graph::prelude::*;

use execution::*;
use prelude::*;
use query::ast as qast;
use schema::ast as sast;

/// Utilities for working with GraphQL query ASTs.
pub mod ast;

/// Options available for query execution.
pub struct QueryOptions<R>
where
    R: Resolver,
{
    /// The logger to use during query execution.
    pub logger: slog::Logger,
    /// The resolver to use.
    pub resolver: R,
}

/// Query.
pub struct Query {
    pub document: q::Document,
    pub schema: s::Document,
    pub variables: Option<HashMap<String, q::Value>>,
}

/// Query result.
pub struct QueryResult {
    pub value: q::Value,
    pub errors: Vec<ExecutionError>,
}

impl QueryResult {
    pub fn new(value: q::Value, errors: Vec<ExecutionError>) -> Self {
        QueryResult { value, errors }
    }

    pub fn add_error(&mut self, e: ExecutionError) {
        self.errors.push(e);
    }
}

impl From<ExecutionError> for QueryResult {
    fn from(e: ExecutionError) -> Self {
        QueryResult::new(q::Value::Null, vec![e])
    }
}

/// Executes a query and returns a result.
pub fn execute_query<R>(query: &Query, options: QueryOptions<R>) -> QueryResult
where
    R: Resolver,
{
    info!(options.logger, "Execute query");

    // Obtain the only operation of the query (fail if there is none or more than one)
    let operation = match qast::get_operation(&query.document, None) {
        Ok(op) => op,
        Err(e) => return QueryResult::from(e),
    };

    match operation {
        // Execute top-level `query { ... }` expressions
        &q::OperationDefinition::Query(q::Query {
            ref selection_set, ..
        }) => execute_root_selection_set(query, options, selection_set, &None),

        // Execute top-level `{ ... }` expressions
        &q::OperationDefinition::SelectionSet(ref selection_set) => {
            execute_root_selection_set(query, options, selection_set, &None)
        }

        // Everything else (e.g. mutations) is unsupported
        _ => QueryResult::from(ExecutionError::NotSupported(
            "Only queries are supported".to_string(),
        )),
    }
}

/// Executes the root selection set of a query.
fn execute_root_selection_set<'a, R>(
    query: &Query,
    options: QueryOptions<R>,
    selection_set: &'a q::SelectionSet,
    initial_value: &Option<q::Value>,
) -> QueryResult
where
    R: Resolver,
{
    // Create an introspection type store and resolver
    let introspection_schema = introspection_schema();
    let introspection_resolver = IntrospectionResolver::new(&options.logger, &query.schema);

    // Create a fresh execution context
    let mut execution = Execution {
        logger: options.logger,
        resolver: Arc::new(options.resolver),
        schema: &query.schema,
        introspection_resolver: Arc::new(introspection_resolver),
        introspection_schema: &introspection_schema,
        introspecting: false,
        document: &query.document,
        fields: vec![],
        errors: vec![],
    };

    // Obtain the root Query type
    match sast::get_root_query_type(&execution.schema) {
        // Execute the root selection set against the root query type
        Some(t) => {
            let value = execution.execute_selection_set(selection_set, t, initial_value);
            QueryResult::new(value, execution.errors.clone())
        }
        // Fail if there is no root Query type
        None => QueryResult::from(ExecutionError::NoRootQueryObjectType),
    }
}
