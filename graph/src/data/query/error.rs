use serde::ser::*;
use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;

use prelude::GraphQLError;

/// Error caused while processing a [Query](struct.Query.html) request.
#[derive(Debug)]
pub enum QueryError<E> {
    EncodingError(FromUtf8Error),
    GraphQLError(E),
}

impl<E> From<FromUtf8Error> for QueryError<E> {
    fn from(e: FromUtf8Error) -> Self {
        QueryError::EncodingError(e)
    }
}

impl<E> From<E> for QueryError<E>
where
    E: GraphQLError,
{
    fn from(e: E) -> Self {
        QueryError::GraphQLError(e)
    }
}

impl<E> Error for QueryError<E>
where
    E: GraphQLError,
{
    fn description(&self) -> &str {
        "Query error"
    }

    fn cause(&self) -> Option<&Error> {
        match self {
            &QueryError::EncodingError(ref e) => Some(e),
            &QueryError::GraphQLError(ref e) => Some(e),
        }
    }
}

impl<E> fmt::Display for QueryError<E>
where
    E: GraphQLError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &QueryError::EncodingError(ref e) => write!(f, "{}", e),
            &QueryError::GraphQLError(ref e) => write!(f, "{}", e),
        }
    }
}

impl<E> Serialize for QueryError<E>
where
    E: GraphQLError,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;

        let msg = match self {
            QueryError::GraphQLError(e) => {
                map.serialize_entry("locations", &e.locations())?;
                format!("{}", self)
            }
            _ => format!("{}", self),
        };

        map.serialize_entry("message", msg.as_str())?;
        map.end()
    }
}
