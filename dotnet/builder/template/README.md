# 将来のbuilder template（Future builder template）

generated .NET builder project template用の予約領域である。将来、generated project ownership、schema-hash-awareな
build/cache contract、または別のproject packagingが必要になった場合に利用する。current production builderはapplicationが
staging workspaceへ一時projectを生成して利用するため、このtemplate自体はruntime sourceではない。
