# 不正fixture（Invalid fixture）

このprojectは、requiredなstable `project.id` fieldを `masterdata.toml` から意図的に除いている。project/config
errorがstructured diagnosticとして報告され、黙って受け入れられないことを検証するために使用する。

追加のinvalid source snippetは、このprojectの隣にあり、parser、validator、code-generationのfocused testに使用する。
対象は、missing/unknown `kind`、duplicate table declaration、duplicate MessagePack field key、invalid generated C#
identifierである。`id` という名前のfieldは、意図的にimplicit primary keyではない。
