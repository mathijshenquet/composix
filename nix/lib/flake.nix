{
  description = "Pure eval-from-lock Cixfile builder";

  outputs = { self }: {
    lib = import ./default.nix;
  };
}
