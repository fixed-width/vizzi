#!/usr/bin/env bash
set -euo pipefail
: "${RELEASE_TAG:?}" "${GITHUB_REPOSITORY:?}" "${GITHUB_SHA:?}" "${RUNNER_TEMP:?}"
python3 scripts/release/package.py validate "$RELEASE_TAG"

verify_tag() {
  local object kind sha
  object=$(gh api "repos/$GITHUB_REPOSITORY/git/ref/tags/$RELEASE_TAG")
  for _ in {1..16}; do
    kind=$(jq -r '.object.type' <<< "$object")
    sha=$(jq -r '.object.sha' <<< "$object")
    if [[ "$kind" == commit ]]; then
      [[ "$sha" == "$GITHUB_SHA" ]] || { echo 'Release tag moved since this build.' >&2; return 1; }
      return 0
    fi
    [[ "$kind" == tag ]] || { echo 'Release ref is not a commit or annotated tag.' >&2; return 1; }
    object=$(gh api "repos/$GITHUB_REPOSITORY/git/tags/$sha")
  done
  echo 'Release tag nesting exceeds allowance.' >&2
  return 1
}

verify_tag
default_branch=$(gh api "repos/$GITHUB_REPOSITORY" --jq .default_branch)
git merge-base --is-ancestor "$GITHUB_SHA" "origin/$default_branch"
if gh release view "$RELEASE_TAG" --repo "$GITHUB_REPOSITORY" >/dev/null 2>&1; then
  echo 'A release already exists for this tag; refusing to replace its assets.' >&2
  exit 1
fi

assets=(dist/*.tar.gz dist/*.vsix dist/SHA256SUMS)
prerelease=()
if [[ "$RELEASE_TAG" == *-* ]]; then prerelease=(--prerelease); fi
gh release create "$RELEASE_TAG" "${assets[@]}" --repo "$GITHUB_REPOSITORY" \
  --verify-tag --draft --title "$RELEASE_TAG" --generate-notes "${prerelease[@]}"

# Check uploaded bytes before publication freezes the assets.
verified=$(mktemp -d "$RUNNER_TEMP/vizzi-release.XXXXXXXX")
gh release download "$RELEASE_TAG" --repo "$GITHUB_REPOSITORY" --dir "$verified"
expected=$(printf '%s\n' "${assets[@]##*/}" | LC_ALL=C sort)
actual=$(find "$verified" -maxdepth 1 -type f -printf '%f\n' | LC_ALL=C sort)
[[ "$actual" == "$expected" ]] || { echo 'Draft asset set differs from the release bundle.' >&2; exit 1; }
cmp dist/SHA256SUMS "$verified/SHA256SUMS"
(cd "$verified" && sha256sum --check SHA256SUMS)
verify_tag
gh release edit "$RELEASE_TAG" --repo "$GITHUB_REPOSITORY" --draft=false

immutable=$(gh api "repos/$GITHUB_REPOSITORY/releases/tags/$RELEASE_TAG" \
  --jq '.draft == false and .immutable == true')
[[ "$immutable" == true ]] || { echo 'Published release is not immutable. Check repository release settings.' >&2; exit 1; }
verify_tag
