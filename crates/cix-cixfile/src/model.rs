use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cixfile {
    pub inputs: BTreeMap<String, Input>,
    pub paths: Vec<Template>,
    pub items: Vec<Item>,
    pub services: BTreeMap<String, Service>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Input {
    pub url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Item {
    Copy { src: String, dst: String },
    File { dst: String, contents: Template },
    Script { dst: String, contents: Template },
    Link { dst: String, target: Template },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Service {
    pub exec: Vec<Template>,
    pub exec_line: usize,
    pub setup: Option<Vec<Template>>,
    pub setup_line: Option<usize>,
    pub env: BTreeMap<String, Env>,
    pub ports: BTreeMap<String, Port>,
    pub listeners: BTreeSet<String>,
    pub dirs: Dirs,
    pub jit: bool,
}

impl Service {
    pub(crate) fn empty() -> Self {
        Self {
            exec: Vec::new(),
            exec_line: 0,
            setup: None,
            setup_line: None,
            env: BTreeMap::new(),
            ports: BTreeMap::new(),
            listeners: BTreeSet::new(),
            dirs: Dirs::default(),
            jit: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Env {
    pub default: Option<Template>,
    pub required: bool,
    pub secret: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Port {
    Env(String),
    Value(u16),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Dirs {
    pub state: BTreeSet<String>,
    pub cache: BTreeSet<String>,
    pub logs: BTreeSet<String>,
    pub config: BTreeSet<String>,
    pub run: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Template {
    pub parts: Vec<TemplatePart>,
}

impl Template {
    pub fn literal(value: impl Into<String>) -> Self {
        Self {
            parts: vec![TemplatePart::Literal(value.into())],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.parts
            .iter()
            .all(|part| matches!(part, TemplatePart::Literal(value) if value.is_empty()))
    }

    pub fn literal_value(&self) -> Option<String> {
        let mut value = String::new();
        for part in &self.parts {
            match part {
                TemplatePart::Literal(part) => value.push_str(part),
                TemplatePart::Package { .. } => return None,
            }
        }
        Some(value)
    }

    pub fn same_value(&self, other: &Self) -> bool {
        self.parts.len() == other.parts.len()
            && self
                .parts
                .iter()
                .zip(&other.parts)
                .all(|(left, right)| match (left, right) {
                    (TemplatePart::Literal(left), TemplatePart::Literal(right)) => left == right,
                    (
                        TemplatePart::Package {
                            namespace: left_namespace,
                            attrpath: left,
                            ..
                        },
                        TemplatePart::Package {
                            namespace: right_namespace,
                            attrpath: right,
                            ..
                        },
                    ) => left_namespace == right_namespace && left == right,
                    _ => false,
                })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Package {
        namespace: String,
        attrpath: String,
        line: usize,
    },
}
