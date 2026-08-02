use std::collections::BTreeMap;

use super::machine::DeclaredName;

pub(super) const LIVE_DIRECTIVES: &[&str] = &[
    "FROM",
    "FETCH",
    "BUILDER",
    "SERVICE",
    "APP",
    "ITEM",
    "COPY",
    "RUN",
    "IMPORT",
    "FILE",
    "LINK",
    "START",
    "START_PRE",
    "ENV",
    "PORT",
    "LISTENER",
    "READINESS",
    "LIVENESS",
    "STATEDIR",
    "CACHEDIR",
    "LOGDIR",
    "DIR",
    "CONFIGDIR",
    "RUNDIR",
    "CLAIM",
    "SHM",
];

struct Migration {
    directive: &'static str,
    replacement: &'static str,
    doc: &'static str,
    #[allow(dead_code)]
    decision: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        directive: "EXEC",
        replacement: "START",
        doc: "docs/cixfile.md#blocks-and-directives",
        decision: "CIP-80",
    },
    Migration {
        directive: "SETUP",
        replacement: "START_PRE",
        doc: "docs/cixfile.md#blocks-and-directives",
        decision: "CIP-80",
    },
    Migration {
        directive: "STATE",
        replacement: "STATEDIR",
        doc: "docs/cixfile.md#role-dirs",
        decision: "D52",
    },
    Migration {
        directive: "LOGS",
        replacement: "LOGDIR",
        doc: "docs/cixfile.md#role-dirs",
        decision: "D52",
    },
    Migration {
        directive: "CONFIG",
        replacement: "CONFIGDIR",
        doc: "docs/cixfile.md#role-dirs",
        decision: "D52",
    },
    Migration {
        directive: "JIT",
        replacement: "CLAIM jit",
        doc: "docs/cixfile.md#claims",
        decision: "D60",
    },
    Migration {
        directive: "EGRESS",
        replacement: "CLAIM egress",
        doc: "docs/cixfile.md#claims",
        decision: "D60",
    },
    Migration {
        directive: "OUTBOUND",
        replacement: "CLAIM egress",
        doc: "docs/cixfile.md#claims",
        decision: "D60",
    },
    Migration {
        directive: "GRANT",
        replacement: "CLAIM <jit|egress>",
        doc: "docs/cixfile.md#claims",
        decision: "CIP-78",
    },
];

struct DockerDirective {
    directive: &'static str,
    fix: &'static str,
}

const DOCKER_DIRECTIVES: &[DockerDirective] = &[
    DockerDirective {
        directive: "WORKDIR",
        fix: "builders already run in /work; delete WORKDIR and adjust RUN paths if needed",
    },
    DockerDirective {
        directive: "CMD",
        fix: "use START <command> inside SERVICE or APP",
    },
    DockerDirective {
        directive: "ENTRYPOINT",
        fix: "use START <command> inside SERVICE or APP",
    },
    DockerDirective {
        directive: "EXPOSE",
        fix: "use PORT <name> = <port> inside SERVICE",
    },
    DockerDirective {
        directive: "USER",
        fix: "delete USER; cix runs artifacts as isolated dynamic users",
    },
    DockerDirective {
        directive: "VOLUME",
        fix: "use STATEDIR, CACHEDIR, LOGDIR, CONFIGDIR, RUNDIR, or DIR according to lifecycle",
    },
    DockerDirective {
        directive: "ARG",
        fix: "use explicit FROM/FETCH inputs or builder ENV",
    },
    DockerDirective {
        directive: "LABEL",
        fix: "remove LABEL; artifact metadata is not a Cixfile directive yet",
    },
    DockerDirective {
        directive: "ADD",
        fix: "use COPY for local files or FETCH for network input",
    },
    DockerDirective {
        directive: "SHELL",
        fix: "invoke an explicit imported shell from RUN",
    },
];

pub(super) fn migration(directive: &str) -> Option<String> {
    MIGRATIONS
        .iter()
        .find(|migration| migration.directive == directive)
        .map(|migration| {
            format!(
                "{} is obsolete; replace it with {}; see {}",
                migration.directive, migration.replacement, migration.doc
            )
        })
}

pub(super) fn unknown_directive(directive: &str) -> String {
    if matches!(directive, "=" | ":") {
        return format!("unexpected {directive:?} before a directive; remove it");
    }
    if let Some(canonical) = LIVE_DIRECTIVES
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(directive))
    {
        return format!(
            "unknown directive {directive:?}; directives are uppercase; write {canonical}"
        );
    }
    if let Some(docker) = nearest(
        directive,
        DOCKER_DIRECTIVES.iter().map(|entry| entry.directive),
    )
    .filter(|candidate| edit_distance(directive, candidate) <= typo_limit(candidate))
    .and_then(|candidate| {
        DOCKER_DIRECTIVES
            .iter()
            .find(|entry| entry.directive == candidate)
    }) {
        let prefix = if docker.directive == directive {
            docker.directive.to_owned()
        } else {
            format!("{directive} looks like Docker's {}", docker.directive)
        };
        return format!(
            "{prefix}, which is Docker vocabulary; {}; see docs/migrate.md#docker-vocabulary",
            docker.fix
        );
    }
    if let Some(candidate) = nearest(directive, LIVE_DIRECTIVES.iter().copied())
        .filter(|candidate| edit_distance(directive, candidate) <= typo_limit(candidate))
    {
        return format!("unknown directive {directive:?}; did you mean {candidate}?");
    }
    format!(
        "unknown directive {directive:?}; see docs/cixfile.md#blocks-and-directives for the supported directives"
    )
}

pub(super) fn binder_suggestion<'a>(
    unknown: &str,
    names: &'a BTreeMap<String, DeclaredName>,
) -> Option<&'a str> {
    nearest(unknown, names.keys().map(String::as_str))
        .filter(|candidate| edit_distance(unknown, candidate) <= typo_limit(candidate))
}

pub(super) fn namespace_suggestion<'a>(
    unknown: &str,
    namespaces: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
    nearest(unknown, namespaces)
        .filter(|candidate| edit_distance(unknown, candidate) <= typo_limit(candidate))
}

fn nearest<'a>(needle: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    candidates.min_by_key(|candidate| (edit_distance(needle, candidate), *candidate))
}

fn typo_limit(candidate: &str) -> usize {
    if candidate.len() >= 8 {
        3
    } else {
        2
    }
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_character) in left.chars().enumerate() {
        let mut current = Vec::with_capacity(right.len() + 1);
        current.push(left_index + 1);
        for (right_index, right_character) in right.iter().enumerate() {
            current.push(std::cmp::min(
                std::cmp::min(current[right_index] + 1, previous[right_index + 1] + 1),
                previous[right_index] + usize::from(left_character != *right_character),
            ));
        }
        previous = current;
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directive_hints_distinguish_typos_case_and_docker_vocabulary() {
        assert!(unknown_directive("SERVIC").contains("did you mean SERVICE"));
        assert!(unknown_directive("from").contains("directives are uppercase"));
        assert!(unknown_directive("WORKDIR").contains("Docker vocabulary"));
        assert!(unknown_directive("EXPOSED").contains("Docker's EXPOSE"));
    }
}
