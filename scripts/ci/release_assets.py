"""Release assets (spec/09-continuous-integration.md CI-2 `release-assets`, rule CON-0.3).

Attaches every `spec/contracts/schemas/*.schema.json` of a release's tag as a release asset,
replacing an asset of the same name, then lists the release assets and fails if any schema of
the tag is missing. Assets that are not schemas of the tag are left as they are.

Reads the schemas from the tag's tree (the argument), the repository from GITHUB_REPOSITORY,
the release tag from RELEASE_TAG and a token that can write releases from GITHUB_TOKEN.

Usage: python3 scripts/ci/release_assets.py TAG-TREE
Exit status: 0 when every schema is attached, 1 otherwise.
"""
import json
import os
import pathlib
import sys
import urllib.error
import urllib.parse
import urllib.request

SCHEMA_DIRECTORY = pathlib.Path("spec/contracts/schemas")
SCHEMA_PATTERN = "*.schema.json"
API = "https://api.github.com"
UPLOADS = "https://uploads.github.com"
PAGE_SIZE = 100


def schema_files(root):
    """Every contract schema of the tree at `root`, in name order."""
    return sorted((root / SCHEMA_DIRECTORY).glob(SCHEMA_PATTERN), key=lambda path: path.name)


def attach(release, schemas):
    """Upload each schema (`name -> bytes`), deleting first an asset of the same name."""
    existing = {asset["name"]: asset["id"] for asset in release.list_assets()}
    for name, content in sorted(schemas.items()):
        if name in existing:
            release.delete_asset(existing[name])
        release.upload_asset(name, content)


def missing(release, schemas):
    """The schema names that the release does not list as assets, in name order."""
    attached = {asset["name"] for asset in release.list_assets()}
    return sorted(name for name in schemas if name not in attached)


def run(release, root):
    """Attach the schemas of the tree at `root` and check them; returns the exit status."""
    schemas = {path.name: path.read_bytes() for path in schema_files(root)}
    if not schemas:
        print(f"no {SCHEMA_PATTERN} under {SCHEMA_DIRECTORY} of the tag", file=sys.stderr)
        return 1
    attach(release, schemas)
    absent = missing(release, schemas)
    for name in absent:
        print(f"missing release asset: {name}", file=sys.stderr)
    if absent:
        return 1
    print(f"attached {len(schemas)} schemas: {', '.join(sorted(schemas))}")
    return 0


class ReleaseApi:
    """The assets of one GitHub release, through the REST API."""

    def __init__(self, repository, token, tag):
        self.repository = repository
        self.token = token
        quoted_tag = urllib.parse.quote(tag, safe="")
        self.release_id = self.call("GET", f"{API}/repos/{repository}/releases/tags/{quoted_tag}")["id"]

    def call(self, method, url, data=None, content_type=None):
        """One REST API call; returns the decoded JSON response, or None when it has no body."""
        headers = {
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {self.token}",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        if content_type is not None:
            headers["Content-Type"] = content_type
        request = urllib.request.Request(url, data=data, method=method, headers=headers)
        with urllib.request.urlopen(request, timeout=60) as response:
            body = response.read()
        return json.loads(body) if body else None

    def list_assets(self):
        """Every asset of the release, across pages."""
        assets = []
        page = 1
        while True:
            url = f"{API}/repos/{self.repository}/releases/{self.release_id}/assets?per_page={PAGE_SIZE}&page={page}"
            batch = self.call("GET", url)
            assets.extend(batch)
            if len(batch) < PAGE_SIZE:
                return assets
            page += 1

    def delete_asset(self, identifier):
        """Delete one asset of the release."""
        self.call("DELETE", f"{API}/repos/{self.repository}/releases/assets/{identifier}")

    def upload_asset(self, name, content):
        """Upload one asset to the release."""
        query = urllib.parse.urlencode({"name": name})
        url = f"{UPLOADS}/repos/{self.repository}/releases/{self.release_id}/assets?{query}"
        self.call("POST", url, data=content, content_type="application/json")


def main(arguments, environment):
    root = pathlib.Path(arguments[0]) if arguments else pathlib.Path(".")
    repository = environment.get("GITHUB_REPOSITORY")
    token = environment.get("GITHUB_TOKEN")
    tag = environment.get("RELEASE_TAG")
    if not repository or not token or not tag:
        print("GITHUB_REPOSITORY, GITHUB_TOKEN and RELEASE_TAG are required", file=sys.stderr)
        return 1
    try:
        return run(ReleaseApi(repository, token, tag), root)
    except urllib.error.HTTPError as error:
        print(f"GitHub REST API error {error.code} on {error.url}: {error.reason}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:], os.environ))
