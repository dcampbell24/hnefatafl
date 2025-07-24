use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Direction {
    Down,
    Right,
}

#[derive(Clone, Debug)]
pub struct Tree<T: Clone + fmt::Debug> {
    pub root: Node<T>,
    path: Vec<Direction>,
}

impl<T: Clone + fmt::Debug> Tree<T> {
    pub fn flatten(&self) -> Vec<T> {
        let mut vec = vec![self.root.node.clone()];
        let mut node = &self.root;

        while let Some(children) = &node.children {
            vec.push(children[0].node.clone());
            node = &children[0];
        }

        vec
    }

    pub fn here(&self) -> Option<T> {
        let mut index = 0;
        let mut here = self.root.clone();
        for direction in &self.path {
            match direction {
                Direction::Down => {
                    let there = here.children?;
                    if index >= there.len() {
                        return None;
                    }
                    here = there[index].clone();
                    index = 0;
                }
                Direction::Right => index += 1,
            }
        }

        Some(here.node)
    }

    pub fn here_node(&self) -> Option<Node<T>> {
        let mut index = 0;
        let mut here = self.root.clone();
        for direction in &self.path {
            match direction {
                Direction::Down => {
                    let there = here.children?;
                    if index >= there.len() {
                        return None;
                    }
                    here = there[index].clone();
                    index = 0;
                }
                Direction::Right => index += 1,
            }
        }

        Some(here)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        let mut len = 1;
        let mut node = &self.root;

        while let Some(children) = &node.children {
            node = &children[0];
            len += 1;
        }

        len
    }

    pub fn new(value: T) -> Self {
        Tree {
            root: Node {
                node: value,
                children: None,
            },
            path: Vec::new(),
        }
    }

    pub fn backward(&mut self) {
        while self.path.ends_with(&[Direction::Right]) {
            self.path.pop();
        }
        if self.path.ends_with(&[Direction::Down]) {
            self.path.pop();
        }
    }

    pub fn backward_all(&mut self) {
        self.path.clear();
    }

    pub fn left(&mut self) {
        if self.path.ends_with(&[Direction::Right]) {
            self.path.pop();
        }
    }

    pub fn right(&mut self) {
        self.path.push(Direction::Right);
    }

    pub fn forward(&mut self) {
        self.path.push(Direction::Down);
    }

    pub fn forward_all(&mut self) {
        if let Some(mut node) = self.here_node() {
            while let Some(children) = node.children {
                self.forward();
                node = children[0].clone();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node<T: Clone + fmt::Debug> {
    pub node: T,
    pub children: Option<Vec<Node<T>>>,
}
