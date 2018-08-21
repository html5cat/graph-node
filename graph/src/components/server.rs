use futures::prelude::*;
use futures::sync::mpsc::{Receiver, Sender};
use futures::sync::oneshot::Canceled;
use serde::ser::*;
use std::error::Error;
use std::fmt;

use prelude::{GraphQLError, Query, QueryError, SchemaProviderEvent};
use util::stream::StreamError;

/// Errors that can occur while processing incoming requests.
#[derive(Debug)]
pub enum GraphQLServerError<E> {
    Canceled(Canceled),
    ClientError(String),
    QueryError(QueryError<E>),
    InternalError(String),
}

impl<E> From<Canceled> for GraphQLServerError<E> {
    fn from(e: Canceled) -> Self {
        GraphQLServerError::Canceled(e)
    }
}

impl<E> From<QueryError<E>> for GraphQLServerError<E> {
    fn from(e: QueryError<E>) -> Self {
        GraphQLServerError::QueryError(e)
    }
}

impl<E> From<&'static str> for GraphQLServerError<E> {
    fn from(s: &'static str) -> Self {
        GraphQLServerError::InternalError(String::from(s))
    }
}

impl<E> From<String> for GraphQLServerError<E> {
    fn from(s: String) -> Self {
        GraphQLServerError::InternalError(s)
    }
}

impl<E> fmt::Display for GraphQLServerError<E>
where
    E: GraphQLError + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &GraphQLServerError::Canceled(_) => write!(f, "Query was canceled"),
            &GraphQLServerError::ClientError(ref s) => write!(f, "{}", s),
            &GraphQLServerError::QueryError(ref e) => write!(f, "{}", e),
            &GraphQLServerError::InternalError(ref s) => write!(f, "{}", s),
        }
    }
}

impl<E> Error for GraphQLServerError<E>
where
    E: GraphQLError + Send + Sync + 'static,
{
    fn description(&self) -> &str {
        "Failed to process the GraphQL request"
    }

    fn cause(&self) -> Option<&Error> {
        match self {
            &GraphQLServerError::Canceled(ref e) => Some(e),
            &GraphQLServerError::ClientError(_) => None,
            &GraphQLServerError::QueryError(ref e) => Some(e),
            &GraphQLServerError::InternalError(_) => None,
        }
    }
}

impl<E> Serialize for GraphQLServerError<E>
where
    E: GraphQLError + Send + Sync + 'static,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let &GraphQLServerError::QueryError(ref e) = self {
            serializer.serialize_some(e)
        } else {
            let mut map = serializer.serialize_map(Some(1))?;
            let msg = format!("{}", self);
            map.serialize_entry("message", msg.as_str())?;
            map.end()
        }
    }
}

/// Common trait for GraphQL server implementations.
pub trait GraphQLServer<E>
where
    E: GraphQLError + Send + Sync + 'static,
{
    type ServeError;

    /// Sender to which others should write whenever the schema that the server
    /// should serve changes.
    fn schema_provider_event_sink(&mut self) -> Sender<SchemaProviderEvent>;

    /// Creates a new Tokio task that, when spawned, brings up the GraphQL server.
    fn serve(
        &mut self,
        port: u16,
    ) -> Result<Box<Future<Item = (), Error = ()> + Send>, Self::ServeError>;
}
