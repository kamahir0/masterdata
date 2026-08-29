# 最小fixture（Minimal fixture）

この固定projectは、`cargo xtask cli`、`cargo xtask gui`、integration smoke testで使用する。
これらのcommandを実行する前に、`target/dev-project` はこのdirectoryから再作成されるため、fixture自体がdevelopment
sessionによって編集されることはない。

schemaとdata fileは意図的に同じsource directoryを共有するが、それぞれが独自の `kind` と `table` を宣言する。
2つ目の `item` 用data fileは、1つのtableを複数のYAML fileに分割できることを示す。
