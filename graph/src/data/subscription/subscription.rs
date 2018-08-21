use graphql_parser::query as q;

use prelude::{QueryVariables, Schema};

pub struct Subscription {
    pub schema: Schema,
    pub document: q::Document,
    pub variables: Option<QueryVariables>,
}
