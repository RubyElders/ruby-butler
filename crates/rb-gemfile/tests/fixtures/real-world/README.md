# Real-world Gemfiles

Copied unchanged from `examples/real-world` on `new-beginning`. Each project includes its original `source.json` with upstream repository, revision, and file hashes, plus its license where present. Lockfiles and the prototype's migrated manifests are excluded: these tests exercise declaration evaluation only.

Foreman's `bundler.d` files and Errbit's `UserGemfile` are included because the Gemfiles evaluate them. ManageIQ's plugin declaration is retained as metadata; plugin code is not installed or executed.

The corpus tests check dependency counts and specific requirements, groups, sources, and require settings. They run offline and do not install any project dependencies.
