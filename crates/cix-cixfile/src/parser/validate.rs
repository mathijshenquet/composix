//! Shared lexical, template, and structural validators for Cixfile directives.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

use crate::*;

use super::diagnostics;
use super::machine::{DeclaredName, ParseError, ServiceMetadata};

pub(super) fn heredoc_delimiter<'a>(
    arguments: &'a str,
    directive: &str,
    line: usize,
    source: &str,
) -> Result<Option<&'a str>, ParseError> {
    if !arguments.starts_with("<<") {
        return Ok(None);
    }
    let fields = exact_fields(arguments, 1, line, source, &format!("{directive} <<EOF"))?;
    let delimiter = fields[0]
        .strip_prefix("<<")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ParseError::new(
                line,
                source,
                format!("{directive} heredoc must use << followed by a delimiter"),
            )
        })?;
    if delimiter.starts_with(['\'', '"']) || delimiter.ends_with(['\'', '"']) {
        return Err(ParseError::new(
            line,
            source,
            format!(
                "{directive} heredoc delimiters are unquoted; write <<{}",
                delimiter.trim_matches(['\'', '"'])
            ),
        ));
    }
    Ok(Some(delimiter))
}

pub(super) fn exact_fields<'a>(
    arguments: &'a str,
    count: usize,
    line: usize,
    source: &str,
    usage: &str,
) -> Result<Vec<&'a str>, ParseError> {
    let fields = arguments.split_whitespace().collect::<Vec<_>>();
    if fields.len() != count {
        if fields.iter().any(|field| field.starts_with('#')) {
            return Err(ParseError::new(
                line,
                source,
                format!("expected {usage}; Cixfile comments must start on their own line"),
            ));
        }
        return Err(ParseError::new(line, source, format!("expected {usage}")));
    }
    Ok(fields)
}

pub(super) fn at_least_one_field<'a>(
    arguments: &'a str,
    line: usize,
    source: &str,
    directive: &str,
) -> Result<Vec<&'a str>, ParseError> {
    let fields = arguments.split_whitespace().collect::<Vec<_>>();
    if fields.is_empty() {
        return Err(ParseError::new(
            line,
            source,
            format!("{directive} requires at least one argument"),
        ));
    }
    Ok(fields)
}

pub(super) fn argv_fields(
    arguments: &str,
    line: usize,
    source: &str,
    directive: &str,
) -> Result<Vec<String>, ParseError> {
    if arguments
        .chars()
        .any(|character| matches!(character, '‘' | '’' | '“' | '”'))
    {
        return Err(ParseError::new(
            line,
            source,
            format!("smart quotes are not valid in {directive}; replace them with ASCII ' or \""),
        ));
    }
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quote = None;
    let mut started = false;
    for character in arguments.chars() {
        match (quote, character) {
            (Some(delimiter), character) if character == delimiter => quote = None,
            (Some(_), character) => field.push(character),
            (None, '\'' | '"') => {
                quote = Some(character);
                started = true;
            }
            (None, character) if character.is_whitespace() => {
                if started {
                    fields.push(std::mem::take(&mut field));
                    started = false;
                }
            }
            (None, character) => {
                field.push(character);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err(ParseError::new(
            line,
            source,
            format!("unterminated quote in {directive} arguments; add the matching quote"),
        ));
    }
    if started {
        fields.push(field);
    }
    if fields.is_empty() {
        return Err(ParseError::new(
            line,
            source,
            format!("{directive} requires at least one argument"),
        ));
    }
    Ok(fields)
}

pub(super) fn build_template(
    input: &str,
    line: usize,
    source: &str,
    heredoc: bool,
    inputs: &BTreeMap<String, Input>,
    names: &BTreeMap<String, DeclaredName>,
) -> Result<Template, ParseError> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if heredoc && bytes[index..].starts_with(b"$${") {
            let Some(close_offset) = input[index + 3..].find('}') else {
                return Err(ParseError::new(
                    line,
                    source,
                    "unterminated $${…} escape in heredoc",
                ));
            };
            let close = index + 3 + close_offset;
            literal.push_str("${");
            literal.push_str(&input[index + 3..close]);
            literal.push('}');
            index = close + 1;
            continue;
        }
        if bytes[index..].starts_with(b"${") {
            let Some(close_offset) = input[index + 2..].find('}') else {
                return Err(ParseError::new(
                    line,
                    source,
                    "unterminated ${…} build-time interpolation",
                ));
            };
            let close = index + 2 + close_offset;
            let reference = &input[index + 2..close];
            if let Some((namespace, attrpath)) = reference.split_once('.') {
                let Some(input) = inputs.get(namespace) else {
                    if names.contains_key(namespace) {
                        return Err(ParseError::new(
                          line,
                          source,
                          format!(
                              "binder {namespace:?} is a source tree; select paths as ${{{namespace}}}/<path>, not with attribute syntax"
                          ),
                      ));
                    }
                    let package_namespaces = inputs
                        .iter()
                        .filter(|(_, input)| input.kind == InputKind::PackageUniverse)
                        .map(|(name, _)| name.as_str())
                        .collect::<Vec<_>>();
                    if let Some(candidate) = diagnostics::namespace_suggestion(
                        namespace,
                        package_namespaces.iter().copied(),
                    ) {
                        return Err(ParseError::new(
                            line,
                            source,
                            format!(
                              "unknown package namespace {namespace:?}; did you mean {candidate:?}?"
                          ),
                        ));
                    }
                    let declared = package_namespaces.join(", ");
                    return Err(ParseError::new(
                      line,
                      source,
                      format!(
                          "unknown package namespace {namespace:?}; declared package namespaces: {declared}"
                      ),
                  ));
                };
                if input.kind == InputKind::Artifact {
                    return Err(ParseError::new(
                      line,
                      source,
                      format!(
                          "cix-item binder {namespace:?} is a source tree; use ${{{namespace}}}/<path>, not attribute syntax; see docs/cixfile.md#inputs"
                      ),
                  ));
                }
                if input.kind != InputKind::PackageUniverse {
                    return Err(ParseError::new(
                        line,
                        source,
                        format!(
                          "FROM source binder {namespace:?} is a tree; use ${{{namespace}}}/<path>"
                      ),
                    ));
                }
                if !valid_attrpath(attrpath) {
                    return Err(ParseError::new(
                      line,
                      source,
                      format!(
                          "package attribute path {attrpath:?} is malformed; use ${{{namespace}.<dot-separated-attrpath>}}"
                      ),
                  ));
                }
                if !literal.is_empty() {
                    parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
                }
                parts.push(TemplatePart::Package {
                    namespace: namespace.to_owned(),
                    attrpath: attrpath.to_owned(),
                    line,
                });
                index = close + 1;
                continue;
            }
            if let Some(input) = inputs.get(reference) {
                if input.kind == InputKind::PackageUniverse {
                    return Err(ParseError::new(
                      line,
                      source,
                      format!(
                          "package universe {reference:?} needs an attribute path, for example ${{{reference}.hello}}"
                      ),
                  ));
                }
            } else if let Some(declaration) = names.get(reference) {
                if declaration.kind == "artifact block" {
                    return Err(ParseError::new(
                      line,
                      source,
                      format!(
                          "artifact block {reference:?} is not a source binder; COPY from a FROM, FETCH, or BUILDER binder"
                      ),
                  ));
                }
            } else {
                if reference == "build" {
                    return Err(ParseError::new(
                        line,
                        source,
                        "no binder named `build`; name your builder: `BUILDER build`",
                    ));
                }
                if let Some(candidate) = diagnostics::binder_suggestion(reference, names) {
                    return Err(ParseError::new(
                        line,
                        source,
                        format!("no binder named {reference:?}; did you mean {candidate:?}?"),
                    ));
                }
                if valid_attrpath(reference) {
                    if let Some(namespace) = inputs
                        .iter()
                        .find(|(_, input)| input.kind == InputKind::PackageUniverse)
                        .map(|(name, _)| name)
                    {
                        return Err(ParseError::new(
                          line,
                          source,
                          format!(
                              "no binder named {reference:?}; for a package, use ${{{namespace}.{reference}}}; binder references are backward-only"
                          ),
                      ));
                    }
                }
                return Err(ParseError::new(
                    line,
                    source,
                    format!("no binder named {reference:?}; binder references are backward-only"),
                ));
            }
            if !literal.is_empty() {
                parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
            }
            parts.push(TemplatePart::Binder {
                name: reference.to_owned(),
                line,
            });
            index = close + 1;
            continue;
        }
        let character = input[index..].chars().next().expect("index in bounds");
        literal.push(character);
        index += character.len_utf8();
    }
    if !literal.is_empty() || parts.is_empty() {
        parts.push(TemplatePart::Literal(literal));
    }
    Ok(Template { parts })
}

pub(super) fn append_template(target: &mut Template, source: Template) {
    for part in source.parts {
        match part {
            TemplatePart::Literal(value) => push_literal(target, &value),
            TemplatePart::Package {
                namespace,
                attrpath,
                line,
            } => target.parts.push(TemplatePart::Package {
                namespace,
                attrpath,
                line,
            }),
            TemplatePart::Binder { name, line } => {
                target.parts.push(TemplatePart::Binder { name, line })
            }
        }
    }
}

pub(super) fn push_literal(template: &mut Template, value: &str) {
    if let Some(TemplatePart::Literal(existing)) = template.parts.last_mut() {
        existing.push_str(value);
    } else {
        template.parts.push(TemplatePart::Literal(value.to_owned()));
    }
}

pub(super) fn reject_build_interpolation(
    input: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if input.contains("${") {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} does not support build-time interpolation"),
        ));
    }
    Ok(())
}

pub(super) fn reject_runtime_variable(
    input: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'$' {
            if bytes.get(index + 1) == Some(&b'{') {
                index += 2;
                continue;
            }
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "runtime $VAR interpolation is only allowed in START and START_PRE, not {label}"
                ),
            ));
        }
        index += 1;
    }
    Ok(())
}

pub(super) fn validate_service_references(
    service: &Service,
    metadata: &ServiceMetadata,
) -> Result<(), ParseError> {
    let commands = [
        (&service.start[..], metadata.start.as_ref()),
        (
            service.start_pre.as_deref().unwrap_or_default(),
            metadata.start_pre.as_ref(),
        ),
    ];
    for (arguments, location) in commands {
        let Some((line, source)) = location else {
            continue;
        };
        for argument in arguments {
            for part in &argument.parts {
                if let TemplatePart::Literal(value) = part {
                    for variable in runtime_variables(value, *line, source)? {
                        if !service.env.contains_key(&variable) {
                            return Err(ParseError::new(
                              *line,
                              source,
                              format!(
                                  "START/START_PRE references undeclared environment variable ${variable}"
                              ),
                          ));
                        }
                    }
                }
            }
        }
    }
    for (name, port) in &service.ports {
        let (line, source) = &metadata.ports[name];
        if let Port::Env(variable) = port {
            let Some(env) = service.env.get(variable) else {
                return Err(ParseError::new(
                    *line,
                    source,
                    format!("PORT references undeclared environment variable ${variable}"),
                ));
            };
            if let Some(default) = &env.default {
                let Some(default) = default.literal_value() else {
                    return Err(ParseError::new(
                        *line,
                        source,
                        format!(
                            "PORT environment variable ${variable} must have a numeric default"
                        ),
                    ));
                };
                let valid = default.parse::<u16>().is_ok_and(|port| port != 0);
                if !valid {
                    return Err(ParseError::new(
                      *line,
                      source,
                      format!("PORT environment variable ${variable} must have a default between 1 and 65535"),
                  ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate_import_template(
    template: &Template,
    inputs: &BTreeMap<String, Input>,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if let [TemplatePart::Binder { name, .. }] = template.parts.as_slice() {
        if inputs
            .get(name)
            .is_some_and(|input| input.kind == InputKind::Artifact)
        {
            return Err(ParseError::new(
              line,
              source,
              format!(
                  "IMPORT cannot use cix-item binder {name:?}; use it as a COPY or LINK source tree; see docs/cixfile.md#inputs"
              ),
          ));
        }
    }
    if matches!(
        template.parts.as_slice(),
        [TemplatePart::Package { .. } | TemplatePart::Binder { .. }]
    ) {
        return Ok(());
    }
    Err(ParseError::new(
        line,
        source,
        "IMPORT requires whole package references such as ${pkgs.coreutils}, without a /bin suffix",
    ))
}

pub(super) fn validate_copy_source(
    template: &Template,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    match template.parts.as_slice() {
      [TemplatePart::Literal(path)] => {
          validate_copy_relative_path(path, "COPY source", line, source)
      }
      [TemplatePart::Package { .. } | TemplatePart::Binder { .. }] => Ok(()),
      [
          TemplatePart::Package { .. } | TemplatePart::Binder { .. },
          TemplatePart::Literal(path),
      ] if path == "/" => Ok(()),
      [
          TemplatePart::Package { .. } | TemplatePart::Binder { .. },
          TemplatePart::Literal(path),
      ] if path.starts_with('/') => {
          validate_copy_relative_path(&path[1..], "COPY source", line, source)
      }
      _ => Err(ParseError::new(
          line,
          source,
          "COPY source must be one bare relative path or one binder/package path such as ${src}/sub/path or ${pkgs.hello}/bin/hello",
      )),
  }
}

pub(super) fn parse_fetch_expect<'a>(
    arguments: &'a str,
    line: usize,
    source: &str,
) -> Result<(Option<String>, &'a str), ParseError> {
    let Some(remainder) = arguments.strip_prefix("EXPECT") else {
        return Ok((None, arguments));
    };
    if remainder
        .chars()
        .next()
        .is_some_and(|character| !character.is_whitespace())
    {
        return Ok((None, arguments));
    }
    let remainder = remainder.trim_start();
    let (hash, command) = remainder.split_once(char::is_whitespace).ok_or_else(|| {
        ParseError::new(
            line,
            source,
            "FETCH EXPECT requires a hash and command: EXPECT <sri-hash> <command…>",
        )
    })?;
    if !hash.starts_with("sha256-") || hash.len() == "sha256-".len() {
        return Err(ParseError::new(
            line,
            source,
            format!("FETCH EXPECT hash must be an SRI sha256 hash, got {hash:?}"),
        ));
    }
    let command = command.trim();
    if command.is_empty() {
        return Err(ParseError::new(
            line,
            source,
            "FETCH EXPECT requires a command after the hash",
        ));
    }
    Ok((Some(hash.to_owned()), command))
}

pub(super) fn runtime_variables(
    input: &str,
    line: usize,
    source: &str,
) -> Result<BTreeSet<String>, ParseError> {
    let bytes = input.as_bytes();
    let mut variables = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        index += 1;
        if index >= bytes.len() || !is_env_start(bytes[index]) {
            return Err(ParseError::new(
                line,
                source,
                "runtime '$' must be followed by an environment variable name",
            ));
        }
        let start = index;
        index += 1;
        while index < bytes.len() && is_env_continue(bytes[index]) {
            index += 1;
        }
        variables.insert(input[start..index].to_owned());
    }
    Ok(variables)
}

pub(super) fn valid_attrpath(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(valid_attr_component)
}

pub(super) fn valid_attr_component(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic() || value == b'_')
        && bytes.all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'-' | b'\''))
}

pub(super) fn validate_namespace(value: &str, line: usize, source: &str) -> Result<(), ParseError> {
    if !valid_attr_component(value) {
        return Err(ParseError::new(
            line,
            source,
            "FROM namespace must be a Nix-style identifier",
        ));
    }
    Ok(())
}

pub(super) fn normalize_input(
    value: &str,
    line: usize,
    source: &str,
) -> Result<(String, InputKind), ParseError> {
    if value == "." || value.starts_with("./") {
        return Ok((value.to_owned(), InputKind::Source));
    }
    let github = value.strip_prefix("github:").is_some_and(|path| {
        let parts = path.split('/').collect::<Vec<_>>();
        (2..=3).contains(&parts.len()) && parts.iter().all(|part| !part.is_empty())
    });
    if github
        || value.starts_with("git+")
        || value.starts_with("path:")
        || value.starts_with("tarball+")
    {
        let kind = if value
            .strip_prefix("github:")
            .is_some_and(|path| path.starts_with("NixOS/nixpkgs/") || path == "NixOS/nixpkgs")
        {
            InputKind::PackageUniverse
        } else {
            InputKind::Source
        };
        return Ok((value.to_owned(), kind));
    }
    match cix_common::Ref::parse(value) {
      Ok(reference) => Ok((reference.display(), InputKind::Artifact)),
      Err(error) => Err(ParseError::new(
          line,
          source,
          format!(
              "FROM input must be a known flakeref (github:, git+, path:, tarball+, ., or ./…) or an index ref with an explicit :tag; {error}"
          ),
      )),
  }
}
pub(super) fn validate_name(
    kind: &str,
    value: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let mut bytes = value.bytes();
    if !bytes
        .next()
        .is_some_and(|value| value.is_ascii_alphanumeric())
        || !bytes.all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'.' | b'-'))
    {
        return Err(ParseError::new(
          line,
          source,
          format!(
              "{kind} name must start with an ASCII letter or digit and contain only letters, digits, '.', '-', or '_'"
          ),
      ));
    }
    Ok(())
}

pub(super) fn validate_env_name(value: &str, line: usize, source: &str) -> Result<(), ParseError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || !is_env_start(bytes[0])
        || !bytes[1..].iter().copied().all(is_env_continue)
    {
        return Err(ParseError::new(
            line,
            source,
            "environment variable name must match [A-Za-z_][A-Za-z0-9_]*",
        ));
    }
    Ok(())
}

pub(super) fn is_env_start(value: u8) -> bool {
    value.is_ascii_alphabetic() || value == b'_'
}

pub(super) fn is_env_continue(value: u8) -> bool {
    is_env_start(value) || value.is_ascii_digit()
}

pub(super) fn validate_item_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if Path::new(value).is_absolute() {
        validate_projected_path(value, label, line, source)
    } else {
        validate_relative_path(value, label, line, source)
    }
}

pub(super) fn normalize_artifact_copy_destination<'a>(
    value: &'a str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<&'a str, ParseError> {
    if value == "/" {
        return Ok(".");
    }
    normalize_artifact_destination(value, label, line, source)
}

pub(super) fn normalize_artifact_destination<'a>(
    value: &'a str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<&'a str, ParseError> {
    let Some(relative) = value.strip_prefix('/') else {
        return Err(ParseError::new(
          line,
          source,
          format!("{label} must be absolute inside the item; write /{value}; see docs/cixfile.md#copy"),
      ));
    };
    validate_projected_path(value, label, line, source)?;
    Ok(relative)
}

pub(super) fn validate_artifact_command_path(
    value: &str,
    directive: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if !value.contains("${") && value.contains('/') && !value.starts_with('/') {
        return Err(ParseError::new(
          line,
          source,
          format!("{directive} path {value:?} must be absolute inside an artifact; write /{value}; see docs/cixfile.md#runtime-path"),
      ));
    }
    Ok(())
}

pub(super) fn validate_projected_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let path = Path::new(value);
    if value == "/" {
        return Err(ParseError::new(
          line,
          source,
          format!("{label} targets a reserved runtime path; choose a path below it; see docs/cixfile.md#copy"),
      ));
    }
    if value.ends_with('/')
        || value.contains("//")
        || value
            .split('/')
            .any(|component| matches!(component, "." | ".."))
    {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} must be a normalized absolute path"),
        ));
    }
    if denied_projected_path(path) {
        return Err(ParseError::new(
          line,
          source,
          format!("{label} targets a reserved runtime path; choose a path below it; see docs/cixfile.md#copy"),
      ));
    }
    Ok(())
}

pub(super) fn denied_projected_path(path: &Path) -> bool {
    [
        "/nix",
        "/proc",
        "/sys",
        "/dev",
        "/run",
        "/var/lib",
        "/var/cache",
        "/var/log",
        "/etc/passwd",
        "/etc/group",
        "/etc/nsswitch.conf",
        "/etc",
        "/usr",
        "/bin",
    ]
    .iter()
    .any(|denied| path == Path::new(denied))
        || path.parent() == Some(Path::new("/"))
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
}

pub(super) fn validate_relative_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} must be a clean relative path"),
        ));
    }
    Ok(())
}

pub(super) fn validate_copy_relative_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if value == "." {
        return Ok(());
    }
    validate_relative_path(value, label, line, source)
}

pub(super) fn validate_role_path(
    value: &str,
    root: &str,
    role: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let path = Path::new(value);
    let relative = path.strip_prefix(root).ok();
    let one_component = relative.is_some_and(|relative| {
        let mut components = relative.components();
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
    });
    if !one_component {
        return Err(ParseError::new(
            line,
            source,
            format!("{role} directory must be exactly one component under {root}"),
        ));
    }
    Ok(())
}
