use graphql_parser::{query as q, Pos};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Serialize)]
pub struct Position {
    line: usize,
    column: usize,
}

impl From<Pos> for Position {
    fn from(pos: Pos) -> Self {
        Position {
            line: pos.line,
            column: pos.column,
        }
    }
}

pub trait GraphQLError: Error + Send {
    fn locations(&self) -> Vec<Position>;
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphQLParseError {
    message: String,
    locations: Vec<Position>,
}

impl From<q::ParseError> for GraphQLParseError {
    fn from(e: q::ParseError) -> Self {
        // Split the inner message into (first line, rest)
        let mut message = format!("{}", e);
        let inner_message = message.replace("query parse error:", "");
        let inner_message = inner_message.trim();
        let parts: Vec<&str> = inner_message.splitn(2, "\n").collect();

        // Find the colon in the first line and split there
        let colon_pos = parts[0].rfind(":").unwrap();
        let (a, b) = parts[0].split_at(colon_pos);

        // Find the line and column numbers and convert them to usize
        let line: usize = a
            .matches(char::is_numeric)
            .collect::<String>()
            .parse()
            .unwrap();
        let column: usize = b
            .matches(char::is_numeric)
            .collect::<String>()
            .parse()
            .unwrap();

        // Generate a list of error locations
        let locations = vec![Position { line, column }];

        // Only use the remainder after the location as the error message
        let message = parts[1].to_string();

        GraphQLParseError { locations, message }
    }
}

impl GraphQLError for GraphQLParseError {
    fn locations(&self) -> Vec<Position> {
        self.locations.clone()
    }
}

impl Error for GraphQLParseError {
    fn description(&self) -> &str {
        "GraphQL parse error"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for GraphQLParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
