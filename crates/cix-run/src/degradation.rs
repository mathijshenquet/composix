use crate::unit::{UnitDefinition, UnitDegradation};

pub(crate) fn without_properties(definition: &UnitDefinition, names: &[&str]) -> UnitDefinition {
    let mut definition = definition.clone();
    definition
        .properties
        .retain(|(name, _)| !names.contains(&name.as_str()));
    definition.text = definition
        .text
        .lines()
        .filter(|line| {
            line.split_once('=')
                .is_none_or(|(name, _)| !names.contains(&name))
        })
        .collect::<Vec<_>>()
        .join("\n");
    definition.text.push('\n');
    definition
}

pub(crate) fn unknown_assignment_names(diagnostics: &str) -> Vec<String> {
    diagnostics
        .lines()
        .filter_map(|line| {
            line.split_once("Unknown assignment: ")
                .and_then(|(_, assignment)| assignment.split_once('=').map(|(name, _)| name))
                .or_else(|| {
                    line.split_once("Unknown key name '")
                        .and_then(|(_, rest)| rest.split_once('\'').map(|(name, _)| name))
                })
                .map(str::to_owned)
        })
        .collect()
}

pub(crate) fn warn_degradations(degradations: &[UnitDegradation]) {
    for degradation in degradations {
        match degradation.property.as_str() {
            "PrivatePIDs=yes" => eprintln!(
                "warning: dropped {}: {}; this service shares the host PID namespace (D36 degraded fallback)",
                degradation.property, degradation.reason
            ),
            "PrivateDevices=yes" => {
                eprintln!(
                    "warning: user manager rejected PrivateDevices isolation ({})",
                    degradation.reason
                );
                eprintln!(
                    "warning: retrying without PrivateDevices; this --user service can access the host device namespace (D13 degraded fallback)"
                );
            }
            property => eprintln!("warning: dropped {property}: {}", degradation.reason),
        }
    }
}

pub(crate) fn without_user_capability_controls(definition: &UnitDefinition) -> UnitDefinition {
    without_properties(
        definition,
        &[
            "AmbientCapabilities",
            "CapabilityBoundingSet",
            "ProtectKernelModules",
            "ProtectKernelLogs",
            "PrivateDevices",
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_unknown_assignment_parser_is_directive_specific() {
        assert_eq!(
            unknown_assignment_names("Unknown assignment: PrivatePIDs=yes\n"),
            vec!["PrivatePIDs"]
        );
    }
}
