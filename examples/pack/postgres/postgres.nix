final: prev: {
  composixPostgresTemplate = prev.runCommand "composix-postgres-template" {
    nativeBuildInputs = [ prev.postgresql ];
  } ''
    LANG=C LC_ALL=C initdb \
      --pgdata="$out" \
      --username=cix \
      --auth-local=trust \
      --auth-host=trust \
      --encoding=UTF8 \
      --no-locale
  '';
}
