use futures::future;
use futures::sync::mpsc::{channel, Receiver, Sender};
use graphql_parser::query as q;
use std::collections::HashMap;
use std::iter::IntoIterator;
use std::marker::PhantomData;
use std::sync::Mutex;

use graph::prelude::{GraphQLRunner as GraphQLRunnerTrait, *};
use graph_graphql::prelude::{
    execute_query, Query as GqlQuery, QueryOptions, QueryResult as GqlQueryResult, StoreResolver,
};

/// Common query runner implementation for The Graph.
pub struct GraphQLRunner<S, E> {
    logger: Logger,
    store: Arc<Mutex<S>>,
    phantom: PhantomData<E>,
}

impl<S, E> GraphQLRunner<S, E>
where
    S: Store + Sized + 'static,
    E: GraphQLError + Send + Sync + 'static,
{
    /// Creates a new query runner.
    pub fn new(logger: &Logger, store: Arc<Mutex<S>>) -> Self {
        GraphQLRunner {
            logger: logger.new(o!("component" => "GraphQLRunner")),
            store: store,
            phantom: PhantomData,
        }
    }
}

impl<S, E> GraphQLRunnerTrait<E> for GraphQLRunner<S, E>
where
    S: Store + Sized + 'static,
    E: GraphQLError + Send + Sync + 'static,
{
    fn run_query(&mut self, query: Query<E>) -> Box<Future<Item = QueryResult<E>, Error = E>> {
        let gql_query = GqlQuery {
            document: query.document.clone(),
            schema: query.schema.document.clone(),
            variables: query.variables.map(HashMap::<String, q::Value>::from),
        };

        let options = QueryOptions {
            logger: self.logger.clone(),
            resolver: StoreResolver::new(&self.logger, self.store.clone()),
        };

        let gql_result = execute_query(&gql_query, options);

        let result = QueryResult {
            data: gql_result.value,
            errors: gql_result
                .errors
                .into_iter()
                .map(QueryError::from)
                .collect(),
        };

        Box::new(future::ok(result))
    }
}
