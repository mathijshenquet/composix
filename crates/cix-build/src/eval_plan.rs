use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactKind, Assembly, BuildStep, Cixfile, Copy, CopyMode, InputKind, LockFile, NodeCommand,
    Template, TemplatePart,
};

pub const EVAL_PLAN_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalPlan {
    pub version: u32,
    #[serde(rename = "cixfileHash")]
    pub cixfile_hash: String,
    pub skeleton: String,
    pub inputs: BTreeMap<String, EvalInput>,
    #[serde(rename = "topLevelFetchCount")]
    pub top_level_fetch_count: usize,
    pub builders: BTreeMap<String, EvalBuilder>,
    pub artifacts: BTreeMap<String, EvalArtifact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalInput {
    pub url: String,
    pub kind: String,
    pub overlays: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalBuilder {
    pub imports: Vec<EvalTemplate>,
    pub environment: BTreeMap<String, String>,
    pub steps: Vec<EvalStep>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum EvalStep {
    Env {
        name: String,
        value: EvalTemplate,
        line: usize,
    },
    Copy {
        copy: EvalCopy,
    },
    Fetch {
        command: EvalCommand,
        environment: BTreeMap<String, EvalTemplate>,
        #[serde(rename = "ignoredEvidence")]
        ignored_evidence: Vec<String>,
        line: usize,
        #[serde(rename = "snapshotNarHash")]
        snapshot_nar_hash: String,
    },
    Run {
        command: EvalCommand,
        environment: BTreeMap<String, EvalTemplate>,
        #[serde(rename = "ignoredEvidence")]
        ignored_evidence: Vec<String>,
        line: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum EvalCommand {
    Legacy {
        command: EvalTemplate,
    },
    Argv {
        argv: Vec<EvalTemplate>,
    },
    Heredoc {
        interpreter: EvalTemplate,
        body: EvalTemplate,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalArtifact {
    pub kind: String,
    pub imports: Vec<EvalTemplate>,
    pub copies: Vec<EvalCopy>,
    pub assembly: Vec<EvalAssembly>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalCopy {
    pub src: EvalTemplate,
    pub dst: String,
    pub mode: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum EvalAssembly {
    File {
        dst: String,
        contents: EvalTemplate,
        line: usize,
    },
    Link {
        dst: String,
        target: EvalTemplate,
        line: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvalTemplate(pub Vec<EvalTemplatePart>);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum EvalTemplatePart {
    Literal { value: String },
    Package { namespace: String, attrpath: String },
    Binder { name: String },
}

impl EvalPlan {
    pub fn from_cixfile(cixfile: &Cixfile, cixfile_hash: String, lock: &LockFile) -> Result<Self> {
        let inputs = cixfile
            .inputs
            .iter()
            .map(|(name, input)| {
                let kind = match input.kind {
                    InputKind::PackageUniverse => "package-universe",
                    InputKind::Source => "source",
                    InputKind::Artifact => "artifact",
                };
                (
                    name.clone(),
                    EvalInput {
                        url: input.url.clone(),
                        kind: kind.to_owned(),
                        overlays: input.overlays.clone(),
                    },
                )
            })
            .collect();
        let builders = cixfile
            .builders
            .iter()
            .map(|(name, builder)| {
                let environment = match lock.builder_dev_envs.get(name) {
                    Some(key) => lock
                        .dev_envs
                        .get(key)
                        .with_context(|| {
                            format!("BUILDER {name} records missing development environment {key}")
                        })?
                        .environment
                        .clone(),
                    None if builder.imports.is_empty() => BTreeMap::new(),
                    None => bail!(
                        "BUILDER {name} has no recorded development environment; run a non-cold build before consuming it through CIP-94"
                    ),
                };
                let steps = builder
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(index, step)| eval_step(name, index, step, lock))
                    .collect::<Result<Vec<_>>>()?;
                Ok((
                    name.clone(),
                    EvalBuilder {
                        imports: builder.imports.iter().map(eval_template).collect::<Result<_>>()?,
                        environment,
                        steps,
                    },
                ))
            })
            .collect::<Result<_>>()?;
        let artifacts = cixfile
            .artifacts
            .iter()
            .map(|(name, artifact)| {
                let kind = match artifact.kind {
                    ArtifactKind::Service => "service",
                    ArtifactKind::App => "app",
                    ArtifactKind::Item => "item",
                };
                Ok((
                    name.clone(),
                    EvalArtifact {
                        kind: kind.to_owned(),
                        imports: artifact
                            .imports
                            .iter()
                            .map(eval_template)
                            .collect::<Result<_>>()?,
                        copies: artifact
                            .copies
                            .iter()
                            .map(eval_copy)
                            .collect::<Result<_>>()?,
                        assembly: artifact
                            .assembly
                            .iter()
                            .map(eval_assembly)
                            .collect::<Result<_>>()?,
                    },
                ))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            version: EVAL_PLAN_VERSION,
            cixfile_hash,
            // Keep this synchronized with nix/lib/build-cixfile.nix. The
            // byte-identity flake check is the executable drift tripwire.
            skeleton: crate::fhs::SKELETON_FINGERPRINT.to_owned(),
            inputs,
            top_level_fetch_count: cixfile.fetches.len(),
            builders,
            artifacts,
        })
    }
}

fn eval_step(builder: &str, index: usize, step: &BuildStep, lock: &LockFile) -> Result<EvalStep> {
    Ok(match step {
        BuildStep::Env {
            name, value, line, ..
        } => EvalStep::Env {
            name: name.clone(),
            value: eval_template(value)?,
            line: *line,
        },
        BuildStep::Copy(copy) => EvalStep::Copy {
            copy: eval_copy(copy)?,
        },
        BuildStep::Fetch {
            command,
            environment,
            ignored_evidence,
            line,
            ..
        } => {
            let prefix = format!("builder:{builder}:{index}-");
            let mut matches = lock
                .fetches
                .iter()
                .filter(|(name, _)| name.starts_with(&prefix));
            let (name, pin) = matches.next().with_context(|| {
                format!(
                    "BUILDER {builder} FETCH at step {} has no lock pin",
                    index + 1
                )
            })?;
            if matches.next().is_some() {
                bail!(
                    "BUILDER {builder} FETCH at step {} has ambiguous lock pins with prefix {prefix:?}",
                    index + 1
                );
            }
            if pin.snapshot_nar_hash.is_empty() {
                bail!(
                    "BUILDER {builder} FETCH pin {name:?} lacks snapshotNarHash; refresh it with --update-lock for CIP-94"
                );
            }
            EvalStep::Fetch {
                command: eval_command(command)?,
                environment: environment
                    .iter()
                    .map(|(name, value)| Ok((name.clone(), eval_template(value)?)))
                    .collect::<Result<_>>()?,
                ignored_evidence: ignored_evidence.iter().cloned().collect(),
                line: *line,
                snapshot_nar_hash: pin.snapshot_nar_hash.clone(),
            }
        }
        BuildStep::Run {
            command,
            environment,
            ignored_evidence,
            line,
            ..
        } => EvalStep::Run {
            command: eval_command(command)?,
            environment: environment
                .iter()
                .map(|(name, value)| Ok((name.clone(), eval_template(value)?)))
                .collect::<Result<_>>()?,
            ignored_evidence: ignored_evidence.iter().cloned().collect(),
            line: *line,
        },
    })
}

fn eval_command(command: &NodeCommand) -> Result<EvalCommand> {
    Ok(match command {
        NodeCommand::Legacy(command) => EvalCommand::Legacy {
            command: eval_template(command)?,
        },
        NodeCommand::Argv(argv) => EvalCommand::Argv {
            argv: argv.iter().map(eval_template).collect::<Result<_>>()?,
        },
        NodeCommand::Heredoc { interpreter, body } => EvalCommand::Heredoc {
            interpreter: eval_template(interpreter)?,
            body: eval_template(body)?,
        },
    })
}

fn eval_copy(copy: &Copy) -> Result<EvalCopy> {
    let mode = match copy.mode {
        CopyMode::Link => "link",
        CopyMode::LinkNormalized => "link-normalized",
        CopyMode::Materialize => "materialize",
    };
    Ok(EvalCopy {
        src: eval_template(&copy.src)?,
        dst: copy.dst.clone(),
        mode: mode.to_owned(),
        line: copy.line,
    })
}

fn eval_assembly(assembly: &Assembly) -> Result<EvalAssembly> {
    Ok(match assembly {
        Assembly::File {
            dst,
            contents,
            line,
        } => EvalAssembly::File {
            dst: dst.clone(),
            contents: eval_template(contents)?,
            line: *line,
        },
        Assembly::Link { dst, target, line } => EvalAssembly::Link {
            dst: dst.clone(),
            target: eval_template(target)?,
            line: *line,
        },
    })
}

fn eval_template(template: &Template) -> Result<EvalTemplate> {
    Ok(EvalTemplate(
        template
            .parts
            .iter()
            .map(|part| match part {
                TemplatePart::Literal(value) => Ok(EvalTemplatePart::Literal {
                    value: value.clone(),
                }),
                TemplatePart::Package {
                    namespace,
                    attrpath,
                    ..
                } => Ok(EvalTemplatePart::Package {
                    namespace: namespace.clone(),
                    attrpath: attrpath.clone(),
                }),
                TemplatePart::Binder { name, .. } => {
                    Ok(EvalTemplatePart::Binder { name: name.clone() })
                }
                TemplatePart::InputMetadata {
                    namespace,
                    attribute,
                    ..
                } => bail!(
                    "unresolved FROM metadata ${{{namespace}.{attribute}}} in CIP-94 eval plan"
                ),
            })
            .collect::<Result<_>>()?,
    ))
}
