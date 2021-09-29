use petgraph::stable_graph::StableDiGraph;
use std::hash::Hash;
use std::collections::HashMap;
use petgraph::Direction;

pub trait Disassembler {
    type Address: Clone + Eq + Hash;
    type EdgeTag: Clone;
    fn next_address(&self, addr: &Self::Address) -> Vec<(Self::Address, Self::EdgeTag)>;
}

pub type ControlFlowGraph<Address, EdgeTag> = StableDiGraph<Vec<Address>, EdgeTag>;



/// Build a control flow graph where each node contains only 0 or 1 instruction.
pub fn build_simple_cfg<Disasm, Address: Clone + Eq + Hash, EdgeTag>(
    disasm: &Disasm,
    entry: &Address,
) -> ControlFlowGraph<Address, EdgeTag>
where
    Disasm: Disassembler<Address = Address, EdgeTag = EdgeTag>,
{
    let mut graph = ControlFlowGraph::new();
    // 这个地方，entry 是个引用，clone 是复制的指针还是内容 ？应该是 clone 完得到的不是个引用
    let mut stack = vec![entry.clone()];
    let mut map = HashMap::new();

    let entry_idx = graph.add_node(vec![entry.clone()]);
    map.insert(entry.clone(), entry_idx);
    while stack.len() > 0 {
        let top_address = stack.pop().unwrap();
        let mut nexts = disasm.next_address(&top_address);
        let idx = map[&top_address];
        while nexts.len() > 0 {
            let (n, e) = nexts.pop().unwrap();
            let idx_2;
            if map.contains_key(&n) {
                idx_2 = map[&n];
            } else {
                idx_2 = graph.add_node(vec![n.clone()]);
                map.insert(n.clone(), idx_2.clone());
                stack.push(n);
            }
            graph.update_edge(idx.clone(), idx_2, e);   
        }
    }

    /*
    for node in graph.node_weights() {
        assert!(node.len() <= 1);
    }*/
    graph
}

fn disjoint_find<T: Hash + Eq + Clone>(map: &mut HashMap<T, T>, idx: T) -> T {

    if map[&idx] == idx {
        idx
    } else {
        let ret = disjoint_find(map, map[&idx].clone());
        // Currently, HashMap does not implement IndexMut, while Vec does.
        // So `map[&idx] == ret.clone()` can not pass the compilation.
        *(map.get_mut(&idx).unwrap()) = ret.clone();
        ret
    }

}

/// Aggregate contiguous nodes in a control flow graph,
/// so that a node can have more than 1 instructions.

pub fn aggregate<Address: Hash + Eq, EdgeTag>(graph: &mut ControlFlowGraph<Address, EdgeTag>) {

    let mut in_nodes_map = HashMap::new();
    let mut out_nodes_map = HashMap::new();
    let mut disjoint_set = HashMap::new();
    for idx in graph.node_indices() {
        if let Some(node) = graph.node_weight(idx.clone()) {
            // 这里会不会有复制一份的问题。C++ 好像是有的，还引出左值右值引用这些东西
            // Rust 里面，变量复制调用 copy，通过 memcpy 实现，即一般没有复杂的逻辑
            // clone 可以定义更复杂的功能比如深拷贝
            let mut vec_out = Vec::new();
            let mut vec_in = Vec::new();
            for item in graph.neighbors_directed(idx.clone(), Direction::Outgoing) {
                vec_out.push(item);
            }
            for item in graph.neighbors_directed(idx.clone(), Direction::Incoming) {
                vec_in.push(item);
            }
            in_nodes_map.insert(idx.clone(), vec_in);
            out_nodes_map.insert(idx.clone(), vec_out);
            disjoint_set.insert(idx.clone(), idx.clone());
        }
    }

    for (idx, in_nodes) in in_nodes_map.iter_mut() {
        if in_nodes.len() == 1 {
            let pre_idx = disjoint_find(&mut disjoint_set, in_nodes[0]);
            if out_nodes_map[&pre_idx].len() == 1 {
                *(disjoint_set.get_mut(&idx).unwrap()) = pre_idx.clone();
                let mut suc_node = graph.remove_node(idx.clone()).unwrap();
                let mut pre_node = graph.node_weight_mut(pre_idx).unwrap();
                pre_node.append(&mut suc_node);

            }
        }
    }

}
