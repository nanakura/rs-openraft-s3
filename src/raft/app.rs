use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use openraft::Config;
use parking_lot::{Mutex, RwLock};

use crate::raft::ExampleRaft;
use crate::raft::NodeId;

#[derive(Clone, Eq, PartialEq)]
pub struct NodeDesc {
    pub node_id: NodeId,
    pub api_addr: String,
    pub rpc_addr: String,
}

impl Ord for NodeDesc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.node_id.cmp(&other.node_id)
    }
}
impl PartialOrd for NodeDesc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
#[derive(Clone)]
pub struct App {
    pub id: NodeId,
    pub api_addr: String,
    pub rpc_addr: String,
    pub raft: ExampleRaft,
    pub key_values: Arc<RwLock<BTreeMap<String, String>>>,
    pub config: Arc<Config>,
    pub nodes: Arc<Mutex<BTreeSet<NodeId>>>,
    pub node_descs: Arc<Mutex<BTreeSet<NodeDesc>>>,
}
