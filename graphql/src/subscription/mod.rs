use graphql_parser::{query as q, schema as s};
use std::collections::HashMap;
use std::sync::Arc;

use graph::prelude::{slog, slog::*, EntityChangeStream, QueryResult, Stream};

use execution::*;
use prelude::*;
use query::ast as qast;
use schema::ast as sast;

/// Options available for subscription execution.
pub struct SubscriptionExecutionOptions<R>
where
    R: Resolver,
{
    /// The logger to use during subscription execution.
    pub logger: slog::Logger,
    /// The resolver to use.
    pub resolver: R,
}

/// Subscription.
pub struct Subscription {
    pub document: q::Document,
    pub schema: s::Document,
    pub variables: Option<HashMap<String, q::Value>>,
}

/// Query result stream.
pub type QueryResultStream = Box<Stream<Item = QueryResult<ExecutionError>, Error = ()> + Send>;

/// Subscription result.
pub struct SubscriptionResult {
    pub stream: Option<QueryResultStream>,
    pub errors: Vec<ExecutionError>,
}

impl SubscriptionResult {
    pub fn new(stream: Option<QueryResultStream>, errors: Vec<ExecutionError>) -> Self {
        SubscriptionResult { stream, errors }
    }

    pub fn add_error(&mut self, e: ExecutionError) {
        self.errors.push(e);
    }
}

impl From<ExecutionError> for SubscriptionResult {
    fn from(e: ExecutionError) -> Self {
        SubscriptionResult::new(None, vec![e])
    }
}

pub fn execute_subscription<R>(
    _subscription: Subscription,
    options: SubscriptionExecutionOptions<R>,
) -> SubscriptionResult
where
    R: Resolver,
{
    info!(options.logger, "Execute subscription");

    //// Obtain the only operation of the subscription (fail if there is none or more than one)
    //let operation = match qast::get_operation(&subscription.document, None) {
    //    Ok(op) => op,
    //    Err(e) => return SubscriptionResult::from(ExecutionError::from(e)),
    //};

    //// Create an introspection type store and resolver
    //let introspection_schema = introspection_schema();
    //let introspection_resolver = IntrospectionResolver::new(&options.logger, &subscription.schema);

    //// Create a fresh execution context
    //let mut ctx = ExecutionContext {
    //    logger: options.logger,
    //    resolver: Arc::new(options.resolver),
    //    schema: &subscription.schema,
    //    introspection_resolver: Arc::new(introspection_resolver),
    //    introspection_schema: &introspection_schema,
    //    introspecting: false,
    //    document: &subscription.document,
    //    fields: vec![],
    //    errors: vec![],
    //};

    //match operation {
    //    // Execute top-level `subscription { ... }` expressions
    //    &q::OperationDefinition::Subscription(ref sub) => {
    //        //let source_stream = match create_source_event_stream(ctx, &sub) {
    //        //    Ok(stream) => stream,
    //        //    Err(e) => return SubscriptionResult::from(e),
    //        //};
    //        //let response_stream = map_source_stream_to_response_stream(ctx, &sub, source_stream);
    //        //SubscriptionResult::new(Some(response_stream))
    //        SubscriptionResult::from(ExecutionError::NotSupported("Too bad".to_string()))
    //    }

    //    // Everything else (e.g. mutations) is unsupported
    //    _ => SubscriptionResult::from(ExecutionError::NotSupported(
    //        "Only subscriptions are supported".to_string(),
    //    )),
    //}

    SubscriptionResult::from(ExecutionError::NotSupported(String::from("What a pity")))
}

//fn create_source_event_stream<'a, R1, R2>(
//    ctx: ExecutionContext<'a, R1, R2>,
//    operation: &q::Subscription,
//) -> Result<EntityChangeStream, ExecutionError>
//where
//    R1: Resolver,
//    R2: Resolver,
//{
//    let subscription_type = match sast::get_root_subscription_type(&ctx.schema) {
//        Some(t) => t,
//        None => return Err(ExecutionError::NoRootSubscriptionObjectType),
//    };
//
//    let grouped_field_set = collect_fields(
//        ctx.clone(),
//        &subscription_type,
//        &operation.selection_set,
//        None,
//    );
//
//    println!("Grouped field set: {:#?}", grouped_field_set);
//
//    Err(ExecutionError::NotSupported("Boo".to_string()))
//}
