use futures::sync::mpsc::{channel, Receiver, Sender};
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

use graph::components::subgraph::RuntimeHostEvent;
use graph::components::subgraph::SubgraphProviderEvent;
use graph::prelude::{SubgraphInstance as SubgraphInstanceTrait, *};

use super::SubgraphInstance;

type InstancesMap = Arc<RwLock<HashMap<SubgraphId, SubgraphInstance>>>;

pub struct SubgraphInstanceManager {
    logger: Logger,
    input: Sender<SubgraphProviderEvent>,
}

impl SubgraphInstanceManager where {
    /// Creates a new runtime manager.
    pub fn new<S, T>(logger: &Logger, store: Arc<Mutex<S>>, host_builder: T) -> Self
    where
        S: Store + 'static,
        T: RuntimeHostBuilder,
    {
        let logger = logger.new(o!("component" => "SubgraphInstanceManager"));

        // Create channel for receiving subgraph provider events.
        let (subgraph_sender, subgraph_receiver) = channel(100);

        // Handle incoming events from the subgraph provider.
        Self::handle_subgraph_events(logger.clone(), subgraph_receiver, store, host_builder);

        SubgraphInstanceManager {
            logger,
            input: subgraph_sender,
        }
    }

    /// Handle incoming events from subgraph providers.
    fn handle_subgraph_events<S, T>(
        logger: Logger,
        receiver: Receiver<SubgraphProviderEvent>,
        store: Arc<Mutex<S>>,
        host_builder: T,
    ) where
        S: Store + 'static,
        T: RuntimeHostBuilder,
    {
        // Subgraph instances
        let instances: InstancesMap = Default::default();

        tokio::spawn(receiver.for_each(move |event| {
            use self::SubgraphProviderEvent::*;

            match event {
                SubgraphAdded(manifest) => {
                    info!(logger, "Subgraph added"; "id" => &manifest.id);
                    Self::handle_subgraph_added(instances.clone(), host_builder.clone(), manifest)
                }
                SubgraphRemoved(id) => {
                    info!(logger, "Subgraph removed"; "id" => &id);
                    Self::handle_subgraph_removed(instances.clone(), id);
                }
            };

            Ok(())
        }));
    }

    fn handle_subgraph_added<T>(
        instances: InstancesMap,
        host_builder: T,
        manifest: SubgraphManifest,
    ) where
        T: RuntimeHostBuilder,
    {
        let id = manifest.id.clone();

        let instance = SubgraphInstance::from_manifest(manifest, host_builder);
        let mut instances = instances.write().unwrap();
        instances.insert(id, instance);
    }

    fn handle_subgraph_removed(instances: InstancesMap, id: SubgraphId) {
        let mut instances = instances.write().unwrap();
        instances.remove(&id);
    }
}

impl EventConsumer<SubgraphProviderEvent> for SubgraphInstanceManager {
    /// Get the wrapped event sink.
    fn event_sink(&self) -> Box<Sink<SinkItem = SubgraphProviderEvent, SinkError = ()> + Send> {
        let logger = self.logger.clone();
        Box::new(self.input.clone().sink_map_err(move |e| {
            error!(logger, "Component was dropped: {}", e);
        }))
    }
}
