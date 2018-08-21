use futures::prelude::*;

use prelude::*;

pub struct SubscriptionResult<E>
where
    E: GraphQLError,
{
    pub stream: Option<Box<Stream<Item = QueryResult<E>, Error = ()>>>,
    pub errors: Option<Vec<SubscriptionError<E>>>,
}

impl<E> SubscriptionResult<E>
where
    E: GraphQLError,
{
    pub fn new(stream: Option<Box<Stream<Item = QueryResult<E>, Error = ()>>>) -> Self {
        SubscriptionResult {
            stream,
            errors: None,
        }
    }

    pub fn add_error(&mut self, e: SubscriptionError<E>) {
        let errors = self.errors.get_or_insert(vec![]);
        errors.push(e);
    }
}

impl<E> From<SubscriptionError<E>> for SubscriptionResult<E>
where
    E: GraphQLError,
{
    fn from(e: SubscriptionError<E>) -> Self {
        let mut result = Self::new(None);
        result.errors = Some(vec![e]);
        result
    }
}

impl<E> From<E> for SubscriptionResult<E>
where
    E: GraphQLError,
{
    fn from(e: E) -> Self {
        let mut result = Self::new(None);
        result.errors = Some(vec![SubscriptionError::from(e)]);
        result
    }
}
