use prelude::*;

use failure::Error;
use futures::sync::oneshot;
use web3::types::Block;
use web3::types::Transaction;

use components::ethereum::EthereumEvent;
use components::ethereum::EthereumEventFilter;

/// Events emitted by a runtime host.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeHostEvent {
    /// An entity should be created or updated.
    EntitySet(StoreKey, Entity, Block<Transaction>),
    /// An entity should be removed.
    EntityRemoved(StoreKey, Block<Transaction>),
}

/// Common trait for runtime host implementations.
pub trait RuntimeHost:
    EventConsumer<(EthereumEvent, oneshot::Sender<Result<(), Error>>)>
    + EventProducer<RuntimeHostEvent>
    + Send
{
    /// The subgraph definition the runtime is for.
    fn subgraph_manifest(&self) -> &SubgraphManifest;

    /// An event filter matching all Ethereum events that this runtime host is interested in.
    fn event_filter(&self) -> EthereumEventFilter;
}

pub trait RuntimeHostBuilder: Send + 'static {
    type Host: RuntimeHost;

    /// Build a new runtime host for a dataset.
    fn build(&mut self, subgraph_manifest: SubgraphManifest, data_source: DataSource)
        -> Self::Host;
}
