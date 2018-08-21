use graphql_parser::query as q;
use std::error::Error;

use prelude::*;

/// The result of running a query.
#[derive(Debug)]
pub struct QueryResult<E>
where
    E: Error,
{
    pub data: q::Value,
    pub errors: Vec<QueryError<E>>,
}

impl<E> QueryResult<E>
where
    E: GraphQLError,
{
    pub fn new(data: q::Value, errors: Vec<QueryError<E>>) -> Self {
        QueryResult { data, errors }
    }
}
