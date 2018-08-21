use serde::ser::*;
use std::error::Error;
use std::fmt;

use prelude::GraphQLError;

/// Error caused while processing a [Subscription](struct.Subscription.html) request.
#[derive(Debug)]
pub enum SubscriptionError<E> {
    GraphQLError(E),
}

impl<E> From<E> for SubscriptionError<E>
where
    E: GraphQLError,
{
    fn from(e: E) -> Self {
        SubscriptionError::GraphQLError(e)
    }
}

impl<E> Error for SubscriptionError<E>
where
    E: GraphQLError,
{
    fn description(&self) -> &str {
        "Subscription error"
    }

    fn cause(&self) -> Option<&Error> {
        match self {
            &SubscriptionError::GraphQLError(ref e) => Some(e),
        }
    }
}

impl<E> fmt::Display for SubscriptionError<E>
where
    E: GraphQLError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &SubscriptionError::GraphQLError(ref e) => write!(f, "{}", e),
        }
    }
}

impl<E> Serialize for SubscriptionError<E>
where
    E: GraphQLError,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;

        let msg = match self {
            _ => format!("{}", self),
        };

        map.serialize_entry("message", msg.as_str())?;
        map.end()
    }
}
