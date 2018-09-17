use failure::Error;
use futures::prelude::*;
use std::sync::{Arc, Mutex};

use graph::prelude::{
    BlockStream as BlockStreamTrait, BlockStreamBuilder as BlockStreamBuilderTrait, EthereumBlock,
    *,
};
use graph::web3::types::{Block, Log, Transaction};

pub struct BlockStream {}

impl BlockStream {
    pub fn new<C>(network: String, subgraph: String, chain_updates: C) -> Self
    where
        C: ChainHeadUpdateListener,
    {
        // TODO: Implement block stream algorithm whenever there is a chain update

        BlockStream {}
    }
}

impl BlockStreamTrait for BlockStream {}

impl Stream for BlockStream {
    type Item = EthereumBlock;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(Async::Ready(None))
    }
}

pub struct BlockStreamBuilder<S, E> {
    store: Arc<Mutex<S>>,
    ethereum: Arc<Mutex<E>>,
    network: String,
}

impl<S, E> Clone for BlockStreamBuilder<S, E> {
    fn clone(&self) -> Self {
        BlockStreamBuilder {
            store: self.store.clone(),
            ethereum: self.ethereum.clone(),
            network: self.network.clone(),
        }
    }
}

impl<S, E> BlockStreamBuilder<S, E>
where
    S: ChainStore,
    E: EthereumAdapter,
{
    pub fn new(store: Arc<Mutex<S>>, ethereum: Arc<Mutex<E>>, network: String) -> Self {
        BlockStreamBuilder {
            store,
            ethereum,
            network,
        }
    }
}

impl<S, E> BlockStreamBuilderTrait for BlockStreamBuilder<S, E>
where
    S: ChainStore,
    E: EthereumAdapter,
{
    type Stream = BlockStream;

    fn from_subgraph(&self, manifest: &SubgraphManifest) -> Self::Stream {
        // Create chain update listener for the network used at the moment.
        //
        // NOTE: We only support a single network at this point, this is why
        // we're just picking the one that was passed in to the block stream
        // builder at the moment
        let chain_update_listener = self
            .store
            .lock()
            .unwrap()
            .chain_head_updates(self.network.as_str());

        // Create the actual network- and subgraph-specific block stream
        BlockStream::new(
            self.network.clone(),
            manifest.id.clone(),
            chain_update_listener,
        )
    }
}
