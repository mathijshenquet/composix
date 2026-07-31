use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cixfile {
    pub inputs: BTreeMap<String, Input>,
    pub fetches: BTreeMap<String, Fetch>,
    pub fetch_order: Vec<String>,
    pub builders: BTreeMap<String, Builder>,
    pub builder_order: Vec<String>,
    pub artifacts: BTreeMap<String, Artifact>,
    pub artifact_order: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Input {
    pub url: String,
    pub kind: InputKind,
    pub line: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputKind {
    PackageUniverse,
    Source,
}

impl Input {
    pub fn is_local(&self) -> bool {
        self.url == "."
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fetch {
    pub expected: Option<String>,
    pub command: Template,
    pub line: usize,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Builder {
    pub imports: Vec<Template>,
    pub steps: Vec<BuildStep>,
    pub line: usize,
}

impl Builder {
    pub(crate) fn empty(line: usize) -> Self {
        Self {
            imports: Vec::new(),
            steps: Vec::new(),
            line,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildStep {
    Env {
        name: String,
        value: String,
        line: usize,
        source: String,
    },
    Copy(Copy),
    Fetch {
        expected: Option<String>,
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
pub struct Copy {
    pub src: Template,
    pub dst: String,
    pub line: usize,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Assembly {
    File { dst: String, contents: Template },
    Link { dst: String, target: Template },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactKind {
    Service,
    App,
}

impl ArtifactKind {
    pub fn manifest_name(self) -> Option<&'static str> {
        match self {
            Self::Service => None,
            Self::App => Some("app"),
        }
    }

    pub fn keyword(self) -> &'static str {
        match self {
            Self::Service => "SERVICE",
            Self::App => "APP",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Artifact {
    pub kind: ArtifactKind,
    pub copies: Vec<Copy>,
    pub assembly: Vec<Assembly>,
    pub service: Service,
    pub line: usize,
}

impl Artifact {
    pub(crate) fn empty(kind: ArtifactKind, line: usize) -> Self {
        Self {
            kind,
            copies: Vec::new(),
            assembly: Vec::new(),
            service: Service::empty(),
            line,
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
    pub grants: BTreeSet<String>,
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
            grants: BTreeSet::new(),
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
                TemplatePart::Package { .. } | TemplatePart::Binder { .. } => return None,
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
                    (
                        TemplatePart::Binder {
                            name: left_name, ..
                        },
                        TemplatePart::Binder {
                            name: right_name, ..
                        },
                    ) => left_name == right_name,
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
    Binder {
        name: String,
        line: usize,
    },
}
