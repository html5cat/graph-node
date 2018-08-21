use futures::future;
use futures::sync::mpsc::{channel, Receiver, Sender};
use graphql_parser::query as gqlq;
use std::collections::BTreeMap;

use graph::prelude::*;

/// A mock `GraphQLRunner`.
pub struct MockGraphQLRunner<S> {
    logger: slog::Logger,
    _store: Arc<S>,
}

impl<S> MockGraphQLRunner<S>
where
    S: Store + Sized + 'static,
{
    /// Creates a new mock `GraphQLRunner`.
    pub fn new(logger: &slog::Logger, store: S) -> Self {
        MockGraphQLRunner {
            logger: logger.new(o!("component" => "MockGraphQLRunner")),
            _store: Arc::new(store),
        }
    }
}

impl<S, E> GraphQLRunner<E> for MockGraphQLRunner<S>
where
    S: Store + Sized + 'static,
    E: GraphQLError + Send + Sync + 'static,
{
    fn run_query(&mut self, query: Query<E>) -> Box<Future<Item = QueryResult<E>, Error = E>> {
        // Here we would access the store.

        let mut data = BTreeMap::new();
        data.insert(
            String::from("allUsers"),
            gqlq::Value::String("placeholder".to_string()),
        );
        data.insert(
            String::from("allItems"),
            gqlq::Value::String("placeholder".to_string()),
        );
        let data = gqlq::Value::Object(data);

        Box::new(future::ok(QueryResult::new(data, vec![])))
    }
}
