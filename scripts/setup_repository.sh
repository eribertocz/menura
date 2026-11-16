#!/usr/bin/env bash
# Configure the GitHub repository as spec/06-development-workflow.md requires.
# Run by the maintainer, once, with an authenticated GitHub CLI (gh auth login).
# Claude sessions cannot run it: their GitHub access does not allow repository settings.
# Usage: scripts/setup_repository.sh OWNER/REPO
set -euo pipefail

repository="${1:?usage: scripts/setup_repository.sh OWNER/REPO}"

echo "Labels"
gh label create spec        --repo "$repository" --color 1D76DB --description "Taken by the specification session" --force
gh label create impl        --repo "$repository" --color 0E8A16 --description "Taken by the implementation session" --force
gh label create spec-defect --repo "$repository" --color D93F0B --description "The specification is ambiguous or incomplete" --force
gh label create ossie-watch --repo "$repository" --color 5319E7 --description "Apache Ossie release or change" --force
gh label create study       --repo "$repository" --color C5DEF5 --description "Development-line study work" --force

echo "Ruleset: protect main (WF-6.1)"
gh api --method POST "repos/$repository/rulesets" --input - <<'JSON'
{
  "name": "protect-main",
  "target": "branch",
  "enforcement": "active",
  "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
  "rules": [
    {"type": "deletion"},
    {"type": "non_fast_forward"},
    {"type": "required_linear_history"},
    {"type": "pull_request", "parameters": {
      "required_approving_review_count": 0,
      "dismiss_stale_reviews_on_push": true,
      "require_code_owner_review": false,
      "require_last_push_approval": false,
      "required_review_thread_resolution": true
    }}
  ]
}
JSON

echo "Ruleset: protect tags (WF-6.1)"
gh api --method POST "repos/$repository/rulesets" --input - <<'JSON'
{
  "name": "protect-tags",
  "target": "tag",
  "enforcement": "active",
  "conditions": {"ref_name": {"include": ["~ALL"], "exclude": []}},
  "rules": [{"type": "deletion"}, {"type": "non_fast_forward"}, {"type": "update"}]
}
JSON

echo "Done. Required status checks are added once the CI workflows of 0.1.0 exist (spec/09-continuous-integration.md)."
