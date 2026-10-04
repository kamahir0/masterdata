# YAML grammar dependency patch

upstream `tree-sitter-yaml 0.7.2`（MIT）のgenerated grammar / scannerを使用する。旧MasterData implementationはinputではない。

scannerのrow / column / serialized positionが`int16_t`であり、32,767行を超えるvalid YAMLを拒否する。reference corpusの2,000 × 20、およびcapacity corpusを扱うため、このposition representationを`int32_t`へ拡張した。grammar / source semanticsは変更しない。

Regression: `large_source_crosses_scanner_line_boundary`。upstreamが同じ制約を解消したreleaseへ移る際はこのpatchを退役できる。
