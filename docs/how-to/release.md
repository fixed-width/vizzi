# Publish a Vizzi release

Maintainer procedure for the Linux/macOS release workflow. Read the
[CI reference](../development/ci.md) for targets and protections.

## Prepare the version

Start from the default branch with CI passing. Set `crates/vizzi/Cargo.toml` to the
version you want to publish, update `Cargo.lock`, and move the applicable changelog
entries under that version and date. Commit and merge these changes before tagging.
The tag must be exactly `v` followed by the binary package version, for example `v0.1.0`.

Check that immutable releases remain enabled using an administrator's GitHub CLI login:

```sh
gh api repos/fixed-width/vizzi/immutable-releases
```

The response must show `enabled: true`. If needed, enable the repository's release
immutability setting before proceeding. Keep it enabled throughout publication.

## Push the tag

From the intended default-branch commit:

```sh
python3 scripts/release/package.py validate v0.1.0
git tag -a v0.1.0 -m "Vizzi 0.1.0"
git push origin v0.1.0
```

The Release workflow runs checks and builds native executables for Linux x86-64/ARM64
and macOS Intel/Apple Silicon. Each archive contains `vizzi`, a short README, build
version/target metadata and the existing third-party notice. Linux uses musl; macOS
uses ad-hoc signing, not Developer ID notarization.

After all four builds pass, the publication job creates a draft with four archives
and `SHA256SUMS`, verifies the uploaded bytes, then publishes it. Check that the release
is marked immutable. GitHub also generates source archives and release attestations.
You can verify an available release using `gh release verify TAG` and verify downloaded
archive checksums with `sha256sum --check SHA256SUMS` (Linux) or
`shasum -a 256 --check SHA256SUMS` (macOS).

## Recover an interrupted attempt

If a build fails, fix the problem in a new commit and choose an appropriate new version;
do not move a published release tag. A rerun can retry transient infrastructure failures
for the same unchanged commit.

If upload or draft verification fails, the draft remains unpublished. The workflow
refuses an existing release, including drafts. Inspect it, confirm it is a draft, and
delete only that unpublished draft before rerunning the failed attempt. Do not replace
published assets or reuse a published version. Published immutable assets and tags cannot
be changed; corrections require another version.
