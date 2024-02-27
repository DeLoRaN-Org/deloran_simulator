use super::{node::Node, path_loss::PathLossModel};

#[derive(Default)]
pub struct World {
    nodes: Vec<Node>,
    epochs: u64,
    path_loss_model: PathLossModel,

}

impl World {
    pub fn new(nodes: Vec<Node>, path_loss_model: PathLossModel) -> World {
        World {
            nodes,
            epochs: 0,
            path_loss_model,
        }
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    pub fn get_nodes(&self) -> &Vec<Node> {
        &self.nodes
    }

    pub fn get_nodes_mut(&mut self) -> &mut Vec<Node> {
        &mut self.nodes
    }

    pub fn get_epochs(&self) -> u64 {
        self.epochs
    }

    pub fn path_loss_model(&self) -> &PathLossModel {
        &self.path_loss_model
    }
}