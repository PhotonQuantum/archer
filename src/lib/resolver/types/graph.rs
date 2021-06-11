use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::Index;

use indexmap::IndexSet;
use itertools::Itertools;
use online_scc_graph::{EdgeEffect as BaseEdgeEffect, Graph as BaseGraph};

use crate::error::{GraphError, Result};

#[derive(Debug, Clone)]
pub struct SCCGraph<T: Hash + Eq + Clone> {
    base: BaseGraph,
    proj: HashMap<T, usize>,
    proj_rev: IndexSet<T>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EdgeEffect<T: Hash + Eq + Clone> {
    None,
    NewEdge(Option<Vec<T>>), // is_cycle
}

impl<T: Hash + Eq + Clone> Default for SCCGraph<T> {
    fn default() -> Self {
        Self {
            base: Default::default(),
            proj: Default::default(),
            proj_rev: Default::default(),
        }
    }
}

impl<T: Hash + Eq + Clone> SCCGraph<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            base: BaseGraph::with_capacity(n),
            proj: HashMap::with_capacity(n),
            proj_rev: IndexSet::with_capacity(n),
        }
    }

    pub fn count(&self) -> usize {
        self.base.count()
    }

    pub fn has_cycle(&self) -> bool {
        self.base.has_cycle()
    }

    pub fn add_node(&mut self, n: T) {
        if !self.proj_rev.contains(&n) {
            let idx = self.base.add_node();
            self.proj.insert(n.clone(), idx);
            self.proj_rev.insert(n);
        }
    }

    pub fn insert(&mut self, i: &T, j: &T) -> Result<EdgeEffect<T>> {
        let ix = self.proj.get(i).ok_or(GraphError::InvalidNode)?;
        let jx = self.proj.get(j).ok_or(GraphError::InvalidNode)?;

        let eff = self
            .base
            .insert(*ix, *jx)
            .map_err(GraphError::SCCGraphError)?;

        Ok(match eff {
            BaseEdgeEffect::None => EdgeEffect::None,
            BaseEdgeEffect::NewEdge(Some(cycles)) => EdgeEffect::NewEdge(Some(
                cycles
                    .into_iter()
                    .map(|p| self.proj_rev.index(p).clone())
                    .collect(),
            )),
            BaseEdgeEffect::NewEdge(None) => EdgeEffect::NewEdge(None),
        })
    }

    pub fn nodes(&self) -> Vec<&T> {
        self.proj_rev.iter().collect()
    }

    pub fn edges(&self) -> Vec<(&T, &T)> {
        self.base
            .edges()
            .map(|(i, j)| (self.proj_rev.index(i), self.proj_rev.index(j)))
            .collect()
    }

    pub fn strongly_connected_components(&self, reversed: bool) -> Vec<Vec<&T>> {
        self.base
            .SCC(reversed)
            .into_iter()
            .map(|component| {
                component
                    .into_iter()
                    .map(|node| self.proj_rev.index(node))
                    .collect()
            })
            .collect()
    }

    pub fn merge(&mut self, other: &Self) -> Result<()> {
        let missing_vertices = other
            .proj_rev
            .difference(&self.proj_rev)
            .cloned()
            .collect_vec();
        for v in missing_vertices {
            self.add_node(v.clone());
        }
        for (i, j) in other.edges() {
            self.insert(i, j)?;
        }
        Ok(())
    }
}

impl<T: Hash + Eq + Clone + Display> SCCGraph<T> {
    pub fn dot(&self) -> String {
        let mut output = String::from("digraph {\n");
        for (idx, data) in self.proj_rev.iter().enumerate() {
            output.push_str(&*format!(
                "    {} [ label = \"{}\" ]\n",
                idx,
                data.to_string()
            ));
        }
        for (i, j) in self.base.edges() {
            output.push_str(&*format!("    {} -> {} [ ]\n", i, j));
        }
        output.push('}');
        output
    }
}
