final: prev: {
  php = prev.php83.withExtensions ({ enabled, all }: enabled ++ (with all; [ calendar gd intl pdo_sqlite zip ]));
}
