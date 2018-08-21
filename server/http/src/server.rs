use futures::sync::mpsc::{channel, Receiver, Sender};
use hyper;
use hyper::Server;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Mutex;

use graph::components::schema::SchemaProviderEvent;
use graph::data::query::Query;
use graph::data::schema::Schema;
use graph::prelude::{GraphQLServer as GraphQLServerTrait, *};

use service::GraphQLService;

/// Errors that may occur when starting the server.
#[derive(Debug)]
pub enum GraphQLServeError {
    OrphanError,
    BindError(hyper::Error),
}

impl Error for GraphQLServeError {
    fn description(&self) -> &str {
        "Failed to start the server"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for GraphQLServeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "OrphanError: No component set up to handle the queries")
    }
}

impl From<hyper::Error> for GraphQLServeError {
    fn from(err: hyper::Error) -> Self {
        GraphQLServeError::BindError(err)
    }
}

/// A GraphQL server based on Hyper.
pub struct GraphQLServer<E, R>
where
    E: GraphQLError + Send + Sync + 'static,
    R: GraphQLRunner<E> + Send + Sync + 'static,
{
    logger: slog::Logger,
    schema_provider_event_sink: Sender<SchemaProviderEvent>,
    schema: Arc<Mutex<Option<Schema>>>,
    graphql_runner: Arc<Mutex<R>>,
    phantom: PhantomData<E>,
}

impl<E, R> GraphQLServer<E, R>
where
    E: GraphQLError + Send + Sync + 'static,
    R: GraphQLRunner<E> + Send + Sync + 'static,
{
    /// Creates a new GraphQL server.
    pub fn new(logger: &slog::Logger, graphql_runner: Arc<Mutex<R>>) -> Self {
        // Create channels for handling incoming events from the schema provider
        let (schema_provider_sink, schema_provider_stream) = channel(100);

        // Create a new GraphQL server
        let mut server = GraphQLServer {
            logger: logger.new(o!("component" => "GraphQLServer")),
            schema_provider_event_sink: schema_provider_sink,
            schema: Arc::new(Mutex::new(None)),
            graphql_runner: graphql_runner,
            phantom: PhantomData,
        };

        // Spawn tasks to handle incoming events from the schema provider
        server.handle_schema_provider_events(schema_provider_stream);

        // Return the new server
        server
    }

    /// Handle incoming events from the schema provider
    fn handle_schema_provider_events(&mut self, stream: Receiver<SchemaProviderEvent>) {
        let logger = self.logger.clone();
        let schema = self.schema.clone();

        tokio::spawn(stream.for_each(move |event| {
            info!(logger, "Received schema provider event");

            let SchemaProviderEvent::SchemaChanged(new_schema) = event;
            let mut schema = schema.lock().unwrap();
            *schema = new_schema;

            Ok(())
        }));
    }
}

impl<E, R> GraphQLServerTrait<E> for GraphQLServer<E, R>
where
    E: GraphQLError + Send + Sync + 'static,
    R: GraphQLRunner<E> + Send + Sync + 'static,
{
    type ServeError = GraphQLServeError;

    fn schema_provider_event_sink(&mut self) -> Sender<SchemaProviderEvent> {
        self.schema_provider_event_sink.clone()
    }

    fn serve(
        &mut self,
        port: u16,
    ) -> Result<Box<Future<Item = (), Error = ()> + Send>, Self::ServeError> {
        let logger = self.logger.clone();

        let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);

        // On every incoming request, launch a new GraphQL service that executes
        // incoming queries using the GraphQL runner.
        let graphql_runner = self.graphql_runner.clone();
        let schema = self.schema.clone();
        let new_service = move || {
            let service = GraphQLService::new(schema.clone(), graphql_runner.clone());
            future::ok::<GraphQLService<E>, hyper::Error>(service)
        };

        // Create a task to run the server and handle HTTP requests
        let task = Server::try_bind(&addr.into())?
            .serve(new_service)
            .map_err(move |e| error!(logger, "Server error"; "error" => format!("{}", e)));

        Ok(Box::new(task))
    }
}
