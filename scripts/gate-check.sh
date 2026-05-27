#!/usr/bin/env bash
# GATE-3 sprint acceptance checker (stub)
# Full implementation: AF-04 / AF-05
# Usage: ./scripts/gate-check.sh [SPRINT_ID]
# Policy: only this script should mark sprints completed in progress.json

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPRINT_ID="${1:-}"

echo "gate-check.sh: GATE-3 stub — full artifact verification not implemented yet."
echo "See docs/sprints/MASTER-PLAN.yaml gate_rules and AF-04/AF-05."

if [[ -z "$SPRINT_ID" ]]; then
  echo "Usage: $0 SPRINT_ID   (e.g. AF-04, GATE-2)"
  exit 1
fi

CARD=""
case "$SPRINT_ID" in
  AF-00)   CARD="$ROOT/docs/sprints/AF-00-security-hardening.yaml" ;;
  AF-00b)  CARD="$ROOT/docs/sprints/AF-00b-code-hygiene.yaml" ;;
  AF-01)   CARD="$ROOT/docs/sprints/AF-01-factory-mvp.yaml" ;;
  AF-02)   CARD="$ROOT/docs/sprints/AF-02-approval-harness.yaml" ;;
  AF-03)   CARD="$ROOT/docs/sprints/AF-03-ws-reliability.yaml" ;;
  AF-04)   CARD="$ROOT/docs/sprints/AF-04-golden-path.yaml" ;;
  AF-04b)  CARD="$ROOT/docs/sprints/AF-04b-executor-routing.yaml" ;;
  AF-05)   CARD="$ROOT/docs/sprints/AF-05-macos-installer.yaml" ;;
  GATE-2)  CARD="$ROOT/docs/sprints/GATE-2-friend-test.yaml" ;;
  AF-06)   CARD="$ROOT/docs/sprints/AF-06-beta-50.yaml" ;;
  AF-07)   CARD="$ROOT/docs/sprints/AF-07-assets-ops.yaml" ;;
  AF-08)   CARD="$ROOT/docs/sprints/AF-08-multi-team-sprint.yaml" ;;
  AF-09)   CARD="$ROOT/docs/sprints/AF-09-public-beta.yaml" ;;
  *)
    echo "Unknown sprint: $SPRINT_ID"
    exit 1
    ;;
esac

if [[ ! -f "$CARD" ]]; then
  echo "Missing sprint card: $CARD"
  exit 1
fi

echo "Sprint card found: $CARD"

# Machine gates that can run today
if [[ "$SPRINT_ID" == "AF-04" || "$SPRINT_ID" == "AF-04b" || "$SPRINT_ID" == "GATE-2" ]]; then
  if command -v cargo >/dev/null 2>&1; then
    echo "Running: cargo test -p nexus_workflow --lib (GATE-2)"
    (cd "$ROOT" && cargo test -p nexus_workflow --lib) || {
      echo "GATE-2 failed: nexus_workflow tests"
      exit 1
    }
  else
    echo "WARN: cargo not found, skipping GATE-2 workflow tests"
  fi
fi

# Artifact checks (examples — extend in AF-04/AF-05)
case "$SPRINT_ID" in
  AF-04)
    if [[ ! -f "$ROOT/docs/GOLDEN-PATH.md" ]]; then
      echo "FAIL: docs/GOLDEN-PATH.md not found (create in AF-04)"
      exit 1
    fi
    ;;
esac

echo ""
echo "STUB: Manual acceptance + artifact checks for $SPRINT_ID are not fully automated."
echo "Do NOT mark sprint completed in yaml/progress.json until real checks pass."
exit 1
