use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cixfile {
    pub inputs: BTreeMap<String, Input>,
    pub paths: Vec<Template>,
    pub caches: Vec<String>,
    pub steps: Vec<BuildStep>,
    pub items: BTreeMap<String, Item>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Input {
    pub url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildStep {
    Copy {
        src: String,
        dst: String,
        line: usize,
        source: String,
    },
    Fetch {
        command: Template,
        line: usize,
        source: String,
    },
    Run {
        command: Template,
        line: usize,
        source: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Assembly {
    File { dst: String, contents: Template },
    Script { dst: String, contents: Template },
    Link { dst: String, target: Template },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Take {
    pub src: Template,
    pub dst: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item {
    pub assembly: Vec<Assembly>,
    pub takes: Vec<Take>,
    pub paths: Vec<Template>,
    pub service: Service,
}

impl Item {
    pub(crate) fn empty() -> Self {
        Self {
            assembly: Vec::new(),
            takes: Vec::new(),
            paths: Vec::new(),
            service: Service::empty(),
        }
    }
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
    pub outbound: bool,
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
            outbound: false,
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
                TemplatePart::Package { .. } | TemplatePart::Build { .. } => return None,
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
                    (TemplatePart::Build { .. }, TemplatePart::Build { .. }) => true,
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
    Build {
        line: usize,
    },
}
