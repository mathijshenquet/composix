{
  src,
  item,
  system ? "x86_64-linux",
}:

let
  lock = builtins.fromJSON (builtins.readFile (src + "/Cixfile.lock"));
  plan = lock.evalPlan or (throw "CIP-94 buildCixfile: Cixfile.lock has no evalPlan; rebuild it with the current cix");

  input = plan.inputs;
  packageUniverseNames = builtins.filter (
    name: input.${name}.kind == "package-universe"
  ) (builtins.attrNames input);
  primaryName =
    if builtins.length packageUniverseNames == 1 then
      builtins.head packageUniverseNames
    else
      throw "CIP-94 buildCixfile: milestone 1 requires exactly one package-universe FROM";

  fetchInput = name:
    let
      declaration = input.${name};
      pin = lock.inputs.${name} or (throw "CIP-94 buildCixfile: lock is missing FROM input ${name}");
      github = builtins.match "github:([^/]+)/([^/]+).*" declaration.url;
    in
    if declaration.kind == "artifact" then
      throw "CIP-94 buildCixfile: artifact FROM inputs are outside milestone 1"
    else if declaration.url == "." || builtins.substring 0 2 declaration.url == "./" then
      sourceRoot
    else if builtins.substring 0 7 declaration.url == "github:" then
      builtins.fetchTree {
        type = "github";
        owner = builtins.elemAt github 0;
        repo = builtins.elemAt github 1;
        inherit (pin) rev narHash;
      }
    else if builtins.substring 0 8 declaration.url == "https://" then
      builtins.fetchTree {
        type = "tarball";
        url = declaration.url;
        inherit (pin) narHash;
      }
    else
      throw "CIP-94 buildCixfile: unsupported FROM URL ${declaration.url}";

  sourceRoot = builtins.path {
    path = src;
    name = "cix-source";
  };
  inputSources = builtins.mapAttrs (name: _: fetchInput name) input;
  universes = builtins.listToAttrs (map (
    name:
    let
      declaration = input.${name};
      overlays = map (
        overlay: import (sourceRoot + builtins.substring 1 (-1) overlay)
      ) declaration.overlays;
    in
    {
      inherit name;
      value = import inputSources.${name} { inherit system overlays; };
    }
  ) packageUniverseNames);
  pkgs = universes.${primaryName};
  lib = pkgs.lib;

  package = namespace: attrpath:
    lib.attrByPath (lib.splitString "." attrpath)
      (throw "CIP-94 buildCixfile: package ${namespace}.${attrpath} does not exist")
      universes.${namespace};
  renderTemplate = binders: template: lib.concatMapStrings (part:
    if part.kind == "literal" then
      part.value
    else if part.kind == "package" then
      builtins.toString (package part.namespace part.attrpath)
    else if part.kind == "binder" then
      builtins.toString binders.${part.name}
    else
      throw "CIP-94 buildCixfile: unknown evalPlan template part ${part.kind}"
  ) template;

  templatePackages = template: builtins.filter (value: value != null) (map (part:
    if part.kind == "package" then package part.namespace part.attrpath else null
  ) template);
  stepPackages = step:
    if step.kind == "copy" then templatePackages step.copy.src
    else if step.kind == "fetch" || step.kind == "run" then templatePackages step.command
    else if step.kind == "env" then templatePackages step.value
    else [ ];

  shellQuote = lib.escapeShellArg;
  envArgs = environment: lib.concatStringsSep " " (lib.mapAttrsToList (
    name: value: "--setenv ${shellQuote name} ${shellQuote value}"
  ) environment);
  exportPrelude = declared: lib.concatStringsSep "" (lib.mapAttrsToList (
    name: value: "export ${name}=${shellQuote value};"
  ) declared);

  baseEnvironment = environment: environment // {
    HOME = "/work";
    LC_ALL = "C";
    PATH = "/bin";
    SOURCE_DATE_EPOCH = "1";
    SSL_CERT_FILE = "/etc/ssl/certs/ca-bundle.crt";
    TMPDIR = "/tmp";
    TZ = "UTC";
  };

  importUnionScript = imports: network: ''
    union="$TMPDIR/cix-import-union"
    mkdir -p "$union"
    merge_import() {
      local source="$1" destination="$2" entry target
      mkdir -p "$destination"
      for entry in "$source"/* "$source"/.[!.]* "$source"/..?*; do
        if [ ! -e "$entry" ] && [ ! -L "$entry" ]; then continue; fi
        target="$destination/''${entry##*/}"
        if [ -d "$entry" ] && [ ! -L "$entry" ]; then
          if [ ! -e "$target" ] && [ ! -L "$target" ]; then mkdir "$target"; fi
          if [ -d "$target" ] && [ ! -L "$target" ]; then merge_import "$entry" "$target"; fi
        elif [ ! -e "$target" ] && [ ! -L "$target" ]; then
          ln -s "$entry" "$target"
        fi
      done
    }
    for package in ${lib.concatMapStringsSep " " shellQuote imports}; do
      for subtree in bin etc share; do
        if [ -d "$package/$subtree" ]; then merge_import "$package/$subtree" "$union/$subtree"; fi
      done
    done
    ${lib.optionalString network ''
      mkdir -p "$union/etc"
      for source in /etc/hosts /etc/nsswitch.conf /etc/resolv.conf; do
        if [ -f "$source" ]; then
          destination="$union/etc/''${source##*/}"
          if [ -e "$destination" ] || [ -L "$destination" ]; then rm -f "$destination"; fi
          cp "$source" "$destination"
        fi
      done
    ''}
  '';

  sandboxCommand = {
    imports,
    environment,
    declared,
    command,
    network,
    line,
  }: ''
    ${importUnionScript imports network}
    test -x "$union/bin/bash" || { echo "CIP-94 buildCixfile: line ${toString line}: RUN/FETCH requires bash in an IMPORTed package" >&2; exit 1; }
    bwrap_args=(
      --die-with-parent --new-session --unshare-user --uid 0 --gid 0
      --unshare-pid --unshare-uts --unshare-ipc --unshare-cgroup
    )
    ${lib.optionalString (!network) ''bwrap_args+=(--unshare-net)''}
    bwrap_args+=(
      --ro-bind /nix/store /nix/store
      --dir /usr --dir /usr/bin --symlink /bin/env /usr/bin/env
    )
    for subtree in bin etc share; do
      if [ -d "$union/$subtree" ]; then bwrap_args+=(--ro-bind "$union/$subtree" "/$subtree"); fi
    done
    if [ ! -d "$union/etc" ]; then bwrap_args+=(--dir /etc); fi
    bwrap_args+=(
      --bind "$out" /work --proc /proc --dev /dev --tmpfs /tmp --chdir /work --clearenv
      ${envArgs environment}
    )
    ${pkgs.bubblewrap}/bin/bwrap "''${bwrap_args[@]}" /bin/bash -c ${shellQuote "umask 022; ${exportPrelude declared}eval \"\$1\""} cix-build ${shellQuote command}
  '';

  restoreScript = snapshot: lib.optionalString (snapshot != null) ''
    cp -a ${shellQuote (builtins.toString snapshot)}/. "$out/"
    chmod -R u+w "$out"
  '';

  builderCopyScript = binders: copy:
    let
      source = renderTemplate binders copy.src;
      destination = copy.dst;
    in
    ''
      copy_source=${shellQuote source}
      copy_destination=${shellQuote destination}
      if [ -d "$copy_source" ] && [ ! -L "$copy_source" ]; then
        if [ "$copy_destination" = . ]; then
          cp -a "$copy_source"/. "$out/"
        else
          mkdir "$out/$copy_destination"
          cp -a "$copy_source"/. "$out/$copy_destination/"
          chmod --reference="$copy_source" "$out/$copy_destination"
        fi
      else
        if [ "$copy_destination" = . ]; then copy_destination="''${copy_source##*/}"; fi
        mkdir -p "$out/$(dirname "$copy_destination")"
        cp -a "$copy_source" "$out/$copy_destination"
      fi
    '';

  buildBuilder = name: builder: binders:
    let
      imports = map (renderTemplate binders) builder.imports;
      allPackages = lib.concatMap templatePackages builder.imports
        ++ lib.concatMap stepPackages builder.steps;
      initial = {
        snapshot = null;
        actions = [ ];
        environment = baseEnvironment builder.environment;
        declared = { };
        fetchNumber = 0;
      };
      state = builtins.foldl' (state: step:
        if step.kind == "env" then
          let value = renderTemplate binders step.value; in state // {
            environment = state.environment // { ${step.name} = value; };
            declared = state.declared // { ${step.name} = value; };
          }
        else if step.kind == "copy" then
          state // { actions = state.actions ++ [ (builderCopyScript binders step.copy) ]; }
        else if step.kind == "run" then
          state // { actions = state.actions ++ [ (sandboxCommand {
            inherit imports;
            inherit (state) environment declared;
            command = renderTemplate binders step.command;
            network = false;
            inherit (step) line;
          }) ]; }
        else if step.kind == "fetch" then
          let
            fetchNumber = state.fetchNumber + 1;
            output = pkgs.stdenvNoCC.mkDerivation {
              name = "cix-fetch-${name}-${toString fetchNumber}";
              dontUnpack = true;
              nativeBuildInputs = [ pkgs.bash pkgs.coreutils pkgs.bubblewrap ] ++ allPackages;
              outputHashMode = "recursive";
              outputHashAlgo = "sha256";
              outputHash = step.snapshotNarHash;
              buildCommand = ''
                set -euo pipefail
                mkdir "$out"
                ${restoreScript state.snapshot}
                ${lib.concatStringsSep "\n" state.actions}
                ${sandboxCommand {
                  inherit imports;
                  inherit (state) environment declared;
                  command = renderTemplate binders step.command;
                  network = true;
                  inherit (step) line;
                }}
              '';
            };
          in
          state // {
            snapshot = output;
            actions = [ ];
            inherit fetchNumber;
          }
        else
          throw "CIP-94 buildCixfile: unknown builder step ${step.kind}"
      ) initial builder.steps;
    in
    pkgs.runCommand "cix-builder-${name}" {
      nativeBuildInputs = [ pkgs.bash pkgs.coreutils pkgs.bubblewrap ] ++ allPackages;
    } ''
      set -euo pipefail
      mkdir "$out"
      ${restoreScript state.snapshot}
      ${lib.concatStringsSep "\n" state.actions}
    '';

  builderNames = builtins.attrNames plan.builders;
  inputBinders = inputSources;
  builderOutputs = if builderNames == [ ] then { } else {
    ${builtins.head builderNames} = buildBuilder (builtins.head builderNames)
      plan.builders.${builtins.head builderNames} inputBinders;
  };
  binders = inputBinders // builderOutputs;

  fhsPackageNames = [ "glibc" "musl" ];
  usesFhsSurface = lib.any (builder: lib.any (template: lib.any (part:
    part.kind == "package" && lib.any (name: lib.elem name fhsPackageNames) (lib.splitString "." part.attrpath)
  ) template) builder.imports) (builtins.attrValues plan.builders);

  artifact = plan.artifacts.${item} or (throw "CIP-94 buildCixfile: unknown ITEM ${item}");
  artifactImports = map (renderTemplate binders) artifact.imports;
  artifactCopySource = copy:
    let source = renderTemplate binders copy.src; in
    if copy.mode == "link-normalized" then builtins.path { path = source; name = "cix-copy"; }
    else source;
  artifactCopies = map artifactCopySource artifact.copies;
  assemblyValues = lib.imap0 (index: entry:
    if entry.kind == "file" then pkgs.writeText "cixfile-file-${toString index}" (renderTemplate binders entry.contents)
    else renderTemplate binders entry.target
  ) artifact.assembly;
  artifactDirectories = lib.unique (map (path: dirOf path) (
    (map (copy: copy.dst) artifact.copies) ++ (map (entry: entry.dst) artifact.assembly)
  ) ++ lib.optionals (artifactImports != [ ]) [ "bin" "etc" "share" ]);

  artifactResult = pkgs.runCommand "cix-item-${item}" {
    preferLocalBuild = true;
    allowSubstitutes = false;
  } ''
    set -eu
    mkdir -p "$out"
    ${lib.optionalString (artifactImports != [ ]) ''
      merge_import() {
        local source="$1" destination="$2" entry target
        mkdir -p "$destination"
        for entry in "$source"/* "$source"/.[!.]* "$source"/..?*; do
          if [ ! -e "$entry" ] && [ ! -L "$entry" ]; then continue; fi
          target="$destination/''${entry##*/}"
          if [ -d "$entry" ] && [ ! -L "$entry" ]; then
            if [ ! -e "$target" ] && [ ! -L "$target" ]; then mkdir "$target"; fi
            if [ -d "$target" ] && [ ! -L "$target" ]; then merge_import "$entry" "$target"; fi
          elif [ ! -e "$target" ] && [ ! -L "$target" ]; then
            ln -s "$entry" "$target"
          fi
        done
      }
      ${lib.concatImapStringsSep "\n" (index: _:
        lib.concatMapStringsSep "\n" (subtree: ''
          if [ -d ${shellQuote (builtins.elemAt artifactImports (index - 1))}/${subtree} ]; then
            merge_import ${shellQuote (builtins.elemAt artifactImports (index - 1))}/${subtree} "$out/${subtree}"
          fi
        '') [ "bin" "etc" "share" ]
      ) artifactImports}
    ''}
    ${lib.concatMapStringsSep "\n" (directory:
      lib.optionalString (directory != ".") "mkdir -p \"$out/${directory}\""
    ) artifactDirectories}
    ${lib.concatImapStringsSep "\n" (index: copy:
      let
        source = builtins.elemAt artifactCopies (index - 1);
        destination = copy.dst;
      in
      ''
        if [ ! -e ${shellQuote (builtins.toString source)} ] && [ ! -L ${shellQuote (builtins.toString source)} ]; then
          echo ${shellQuote "line ${toString copy.line}: COPY source does not exist"} >&2; exit 1
        fi
        ${
          if copy.mode == "link" || copy.mode == "link-normalized" then
            "ln -s ${shellQuote (builtins.toString source)} \"$out/${destination}\""
          else if destination == "." then
            "if [ -d ${shellQuote (builtins.toString source)} ]; then cp -a ${shellQuote (builtins.toString source)}/. \"$out/\"; else cp -a ${shellQuote (builtins.toString source)} \"$out/\"; fi"
          else
            "if [ -d ${shellQuote (builtins.toString source)} ]; then mkdir -p \"$out/${destination}\"; cp -a ${shellQuote (builtins.toString source)}/. \"$out/${destination}/\"; chmod -R u+w \"$out/${destination}\"; else cp -a ${shellQuote (builtins.toString source)} \"$out/${destination}\"; fi"
        }
      ''
    ) artifact.copies}
    ${lib.concatImapStringsSep "\n" (index: entry:
      let value = builtins.elemAt assemblyValues (index - 1); in
      if entry.kind == "file" then
        "install -m 0644 ${shellQuote (builtins.toString value)} \"$out/${entry.dst}\""
      else
        "ln -s ${shellQuote (builtins.toString value)} \"$out/${entry.dst}\""
    ) artifact.assembly}
  '';

  # Must match crates/cix-build/src/fhs.rs:SKELETON_FINGERPRINT. The FHS
  # aliases named by this fingerprint are deliberately rejected below.
  supportedSkeleton = "v2:/usr/bin/env->/bin/env;x86_64:/lib64/ld-linux-x86-64.so.2->/lib/cix-loaders/ld-linux-x86-64.so.2,/lib/ld-musl-x86_64.so.1->/lib/cix-loaders/ld-musl-x86_64.so.1";
in
if plan.version != 1 then
  throw "CIP-94 buildCixfile: unsupported evalPlan version ${toString plan.version}"
else if plan.cixfileHash != builtins.hashFile "sha256" (src + "/Cixfile") then
  throw "CIP-94 buildCixfile: Cixfile changed after Cixfile.lock; rebuild the lock with cix"
else if plan.skeleton != supportedSkeleton then
  throw "CIP-94 buildCixfile: builder skeleton drifted; update the Nix replay and byte-identity check together"
else if plan.topLevelFetchCount != 0 then
  throw "CIP-94 buildCixfile: top-level FETCH is outside milestone 1; use one BUILDER"
else if builtins.length builderNames > 1 then
  throw "CIP-94 buildCixfile: multi-BUILDER graphs are outside milestone 1"
else if artifact.kind != "item" then
  throw "CIP-94 buildCixfile: SERVICE/APP manifest generation is outside milestone 1; select an ITEM"
else if usesFhsSurface then
  throw "CIP-94 buildCixfile: this builder imports the CIP-95 FHS loader surface, which CIP-94 milestone 1 cannot reproduce; use cix build --cold"
else
  artifactResult
