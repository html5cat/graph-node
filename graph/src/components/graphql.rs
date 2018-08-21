use futures::Future;

use prelude::{GraphQLError, Query, QueryResult};

/// Common trait for components that run queries against a [Store](../store/trait.Store.html).
pub trait GraphQLRunner<E>
where
    E: GraphQLError + Send + Sync + 'static,
{
    // Sender to which others can write queries that need to be run.
    fn run_query(&mut self, query: Query<E>) -> Box<Future<Item = QueryResult<E>, Error = E>>;
}
