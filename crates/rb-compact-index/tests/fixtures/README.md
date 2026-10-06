# Compact-index fixtures

Captured from public metadata endpoints on 2026-10-06. Reduced fixtures retain
complete selected records in server order and the original header, if present.
No record contents or archive checksums were changed.

| Fixture | Source | Selection |
| --- | --- | --- |
| `rubygems-nokogiri.info` | https://rubygems.org/info/nokogiri | Java prerelease; pure Ruby, GNU/Linux, musl and Windows releases |
| `rubygems-activerecord-import.info` | https://rubygems.org/info/activerecord-import | 0.2.8.rc1, 0.2.8 and 2.2.0; repeated dependency requirements |
| `rubygems-rake.info` | https://rubygems.org/info/rake | 0.4.11 and 13.4.2; empty dependencies |
| `gem-coop-rake.info` | https://gem.coop/info/rake | Same versions to check mirror equivalence |
| `gem-coop-dry-core.info` | https://gem.coop/@dry/info/dry-core | 0.1.0 and 1.2.0; headerless namespace, licenses and publication dates |
| `gem-coop-oaken.info` | https://gem.coop/@kaspth/info/oaken | Unchanged complete response; headerless namespace |

HTTP regression tests serve these bodies locally. Digest headers are computed
for the fixture body, not copied from the larger original response. Redirects,
304 revalidation, authentication errors and malformed digests are synthetic
HTTP scenarios, not claimed captures from a particular provider. Full-index
fallback tests generate tiny Marshal indexes and gem archives locally; binary
archives are not truncated. No credentials, gems or large registry indexes are
stored here, and tests do not access the public services.
