use crate::unit::UnitDefinition;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Default)]
pub struct DependencyGraph {
    units: BTreeMap<String, UnitDefinition>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("unit {0} already exists")]
    DuplicateUnit(String),
    #[error("unit {0} was not found")]
    UnknownUnit(String),
    #[error("dependency cycle detected: {0:?}")]
    Cycle(Vec<String>),
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_unit(&mut self, unit: UnitDefinition) -> Result<(), GraphError> {
        let name = unit.unit.name.clone();
        if self.units.contains_key(&name) {
            return Err(GraphError::DuplicateUnit(name));
        }
        self.units.insert(name, unit);
        Ok(())
    }

    pub fn start_order(&self, targets: &[String]) -> Result<Vec<String>, GraphError> {
        let mut included = BTreeSet::new();
        for target in targets {
            self.collect_start_set(target, &mut included)?;
        }

        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut order = Vec::new();

        for unit in included.iter() {
            self.visit(unit, &included, &mut visiting, &mut visited, &mut order)?;
        }

        Ok(order)
    }

    fn collect_start_set(
        &self,
        unit: &str,
        included: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let definition = self
            .units
            .get(unit)
            .ok_or_else(|| GraphError::UnknownUnit(unit.to_string()))?;

        if !included.insert(unit.to_string()) {
            return Ok(());
        }

        for dependency in definition
            .dependencies
            .requires
            .iter()
            .chain(definition.dependencies.wants.iter())
        {
            self.collect_start_set(dependency, included)?;
        }

        Ok(())
    }

    fn visit(
        &self,
        unit: &str,
        included: &BTreeSet<String>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), GraphError> {
        if visited.contains(unit) {
            return Ok(());
        }
        if !visiting.insert(unit.to_string()) {
            let mut cycle = visiting.iter().cloned().collect::<Vec<_>>();
            cycle.push(unit.to_string());
            return Err(GraphError::Cycle(cycle));
        }

        for before in self.ordering_dependencies(unit, included)? {
            self.visit(&before, included, visiting, visited, order)?;
        }

        visiting.remove(unit);
        visited.insert(unit.to_string());
        order.push(unit.to_string());
        Ok(())
    }

    fn ordering_dependencies(
        &self,
        unit: &str,
        included: &BTreeSet<String>,
    ) -> Result<Vec<String>, GraphError> {
        let definition = self
            .units
            .get(unit)
            .ok_or_else(|| GraphError::UnknownUnit(unit.to_string()))?;

        let mut dependencies = BTreeSet::new();
        for after in &definition.dependencies.after {
            if included.contains(after) {
                dependencies.insert(after.clone());
            }
        }

        for (other_name, other_definition) in &self.units {
            if included.contains(other_name)
                && other_definition
                    .dependencies
                    .before
                    .iter()
                    .any(|before| before == unit)
            {
                dependencies.insert(other_name.clone());
            }
        }

        Ok(dependencies.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::parse_unit_toml;

    fn unit(name: &str, body: &str) -> UnitDefinition {
        parse_unit_toml(&format!(
            r#"
[unit]
name = "{name}"

[exec]
start = ["/bin/true"]

{body}
"#
        ))
        .unwrap()
    }

    #[test]
    fn start_order_includes_required_units_before_target() {
        let mut graph = DependencyGraph::new();
        graph.add_unit(unit("network.target", "")).unwrap();
        graph
            .add_unit(unit(
                "sshd",
                r#"
[dependencies]
requires = ["network.target"]
"#,
            ))
            .unwrap();

        let order = graph.start_order(&["sshd".to_string()]).unwrap();

        assert_eq!(order, vec!["network.target", "sshd"]);
    }

    #[test]
    fn start_order_respects_after_edges() {
        let mut graph = DependencyGraph::new();
        graph.add_unit(unit("network.target", "")).unwrap();
        graph
            .add_unit(unit(
                "sshd",
                r#"
[dependencies]
after = ["network.target"]
"#,
            ))
            .unwrap();

        let order = graph
            .start_order(&["sshd".to_string(), "network.target".to_string()])
            .unwrap();

        assert_eq!(order, vec!["network.target", "sshd"]);
    }

    #[test]
    fn start_order_respects_before_edges() {
        let mut graph = DependencyGraph::new();
        graph
            .add_unit(unit(
                "early",
                r#"
[dependencies]
before = ["late"]
"#,
            ))
            .unwrap();
        graph.add_unit(unit("late", "")).unwrap();

        let order = graph
            .start_order(&["late".to_string(), "early".to_string()])
            .unwrap();

        assert_eq!(order, vec!["early", "late"]);
    }

    #[test]
    fn cycles_are_reported() {
        let mut graph = DependencyGraph::new();
        graph
            .add_unit(unit(
                "a",
                r#"
[dependencies]
after = ["b"]
"#,
            ))
            .unwrap();
        graph
            .add_unit(unit(
                "b",
                r#"
[dependencies]
after = ["a"]
"#,
            ))
            .unwrap();

        let error = graph
            .start_order(&["a".to_string(), "b".to_string()])
            .unwrap_err();

        assert!(matches!(error, GraphError::Cycle(_)));
    }

    #[test]
    fn unknown_target_is_reported() {
        let graph = DependencyGraph::new();

        let error = graph.start_order(&["missing".to_string()]).unwrap_err();

        assert_eq!(error, GraphError::UnknownUnit("missing".to_string()));
    }
}
