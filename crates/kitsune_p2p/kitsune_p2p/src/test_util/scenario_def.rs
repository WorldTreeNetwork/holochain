//! Declarative definition of multi-conductor sharded scenarios.

use std::collections::{BTreeSet, HashSet};

use kitsune_p2p_types::dht_arc::ArcInterval;

/// A "coarse" DHT location specification, defined at a lower resolution
/// than the full u32 space, for convenience in more easily covering the entire
/// space in tests.
type CoarseLoc = i32;

/// Abstract representation of the instantaneous state of a sharded network
/// with multiple conductors. Useful for setting up multi-node test scenarios,
/// and for deriving the expected final state after reaching consistency.
///
/// NB: The concrete scenarios derived from this definition will in general break a rule:
///     The agent arcs will not be centered on the agent's DHT location.
///
/// Thus, rather than dealing with hash types directly, this representation
/// deals only with locations.
///
/// Thus, note that for simplicity's sake, it's impossible to specify two ops
/// at the same location, which is possible in reality, but rare, and should
/// have no bearing on test results. (TODO: test this case separately)
pub struct ScenarioDef<const N: usize> {
    /// The "nodes" (in Holochain, "conductors") participating in this scenario
    pub nodes: [ScenarioDefNode; N],

    /// Specifies which other nodes are present in the peer store of each node.
    /// The array index matches the array defined in `ShardedScenario::nodes`.
    pub peer_matrix: PeerMatrix<N>,

    /// Represents latencies between nodes, to be simulated.
    /// If None, all latencies are zero.
    pub _latency_matrix: LatencyMatrix<N>,

    /// DhtLocations may be specified in a smaller set of integers than the full
    /// u32 space, for convenience. This number specifies the size of the space
    /// to work with.
    ///
    /// The `HashedFixtures` construct works with a u8 space, and in such cases
    /// this `resolution` should be set to `u8::MAX`
    ///
    /// Any reference to a DHT arc endpoint defined in a scenario will be
    /// multiplied by a factor to properly map the lower-resolution location
    /// into the full u32 location space.
    ///
    /// e.g. for a u8 resolution, the multiplicative factor is `u32::MAX / u8::MAX`
    pub resolution: u32,
}

impl<const N: usize> ScenarioDef<N> {
    /// Constructor
    pub fn new(nodes: [ScenarioDefNode; N], peer_matrix: PeerMatrix<N>) -> Self {
        Self::new_with_latency(nodes, peer_matrix, None)
    }

    fn new_with_latency(
        nodes: [ScenarioDefNode; N],
        peer_matrix: PeerMatrix<N>,
        _latency_matrix: LatencyMatrix<N>,
    ) -> Self {
        Self {
            // Resolution is hard-coded for now, but can be modified if ever
            // needed
            resolution: u8::MAX as u32,
            nodes,
            peer_matrix,
            _latency_matrix,
        }
    }
}

/// An individual node in a sharded scenario.
/// The only data needed is the list of local agents.
pub struct ScenarioDefNode {
    /// The agents local to this node
    pub agents: HashSet<ScenarioDefAgent>,
}

impl ScenarioDefNode {
    /// Constructor
    pub fn new<A: IntoIterator<Item = ScenarioDefAgent>>(agents: A) -> Self {
        Self {
            agents: agents.into_iter().collect(),
        }
    }
}

/// An individual agent on a node in a sharded scenario
#[derive(PartialEq, Eq, Hash)]
pub struct ScenarioDefAgent {
    /// The storage arc for this agent
    arc: (CoarseLoc, CoarseLoc),
    /// The ops stored by this agent
    pub ops: BTreeSet<CoarseLoc>,
}

impl ScenarioDefAgent {
    /// Constructor
    pub fn new<O: IntoIterator<Item = CoarseLoc>>(arc: (CoarseLoc, CoarseLoc), ops: O) -> Self {
        Self {
            arc,
            ops: ops.into_iter().collect(),
        }
    }

    /// Produce an ArcInterval in the u32 space from the lower-resolution
    /// definition, based on the resolution defined in the ScenarioDef which
    /// is passed in
    pub fn arc(&self, resolution: u32) -> ArcInterval {
        let start = rectify_index(resolution, self.arc.0);
        let end = rectify_index(resolution, self.arc.1 + 1) - 1;
        ArcInterval::new(start, end)
    }
}

/// A latency matrix, defining a simulated latency between any two nodes,
/// i.e. latency_matrix[A][B] is the latency in milliseconds for communication
/// from node A to node B.
/// To represent partitions, just set the latency very high (`u32::MAX`).
/// If None, all latencies are zero.
pub type LatencyMatrix<const N: usize> = Option<[[u32; N]; N]>;

/// Specifies which other nodes are present in the peer store of each node.
/// The array index matches the array defined in `ShardedScenario::nodes`.
pub enum PeerMatrix<const N: usize> {
    /// All nodes know about all other nodes
    Full,
    /// Each index of the matrix is a hashset of other indices: The node at
    /// this index knows about the other nodes at the indices in the hashset.
    Sparse([HashSet<usize>; N]),
}

impl<const N: usize> PeerMatrix<N> {
    /// Construct a full matrix (full peer connectivity)
    pub fn full() -> Self {
        Self::Full
    }

    /// Construct a sparse matrix by the given nodes.
    /// More convenient than constructing the enum variant directly, since the
    /// inner collection type is a slice rather than a HashSet.
    pub fn sparse<'a>(matrix: [&'a [usize]; N]) -> Self {
        use std::convert::TryInto;
        Self::Sparse(
            matrix
                // TODO: when array map stabilizes, the node.clone() below
                // can be removed
                .iter()
                .map(|node| {
                    node.clone()
                        .into_iter()
                        .map(|u| u.clone())
                        .collect::<HashSet<_>>()
                })
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        )
    }
}

/// Map a signed index into an unsigned index
pub fn rectify_index(num: u32, i: i32) -> u32 {
    let num = num as i32;
    if i >= num || i <= -num {
        panic!(
            "attempted to rectify an out-of-bounds index: |{}| >= {}",
            i, num
        );
    }
    if i < 0 {
        (num + i) as u32
    } else {
        i as u32
    }
}

/// Just construct a scenario to illustrate/experience how it's done
#[test]
fn constructors() {
    use ScenarioDefAgent as Agent;
    use ScenarioDefNode as Node;
    let ops: Vec<CoarseLoc> = (-10..11).map(i32::into).collect();
    let nodes = [
        Node::new([
            Agent::new((ops[0], ops[2]), [ops[0], ops[1]]),
            Agent::new((ops[3], ops[4]), [ops[3], ops[4]]),
        ]),
        Node::new([
            Agent::new((ops[0], ops[2]), [ops[5], ops[7]]),
            Agent::new((ops[3], ops[4]), [ops[6], ops[9]]),
        ]),
    ];
    let _scenario = ScenarioDef::new(nodes, PeerMatrix::sparse([&[1], &[]]));
}