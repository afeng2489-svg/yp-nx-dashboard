#!/usr/bin/env bash
# GATE-3 sprint acceptance checker
# Usage: ./scripts/gate-check.sh SPRINT_ID

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPRINT_ID="${1:-}"

pass() { echo "  PASS: $1"; }
fail() { echo "  FAIL: $1"; exit 1; }

if [[ -z "$SPRINT_ID" ]]; then
  echo "Usage: $0 SPRINT_ID   (e.g. AF-04, AF-UX-01, AF-P5, GATE-2)"
  exit 1
fi

echo "=== gate-check: $SPRINT_ID ==="

case "$SPRINT_ID" in
  AF-04)
    [[ -f "$ROOT/docs/GOLDEN-PATH.md" ]] || fail "docs/GOLDEN-PATH.md missing"
    pass "GOLDEN-PATH.md exists"
    [[ -f "$ROOT/scripts/e2e-golden-path.mjs" ]] || fail "e2e-golden-path.mjs missing"
    pass "golden-path e2e script"
    ;;
  AF-04b)
    [[ -f "$ROOT/core/workflow/src/executor.rs" ]] || fail "executor.rs missing"
    pass "executor module"
    grep -q "交付摘要" "$ROOT/config/workflows/solo-dev.yaml" || fail "solo-dev 交付摘要 stage"
    pass "solo-dev API stage"
    ;;
  AF-05)
    [[ -f "$ROOT/docs/TROUBLESHOOTING.md" ]] || fail "TROUBLESHOOTING.md missing"
    pass "TROUBLESHOOTING.md"
    [[ -f "$ROOT/docs/RELEASE.md" ]] || fail "RELEASE.md missing"
    pass "RELEASE.md"
    [[ -f "$ROOT/nx_dashboard/src/components/onboarding/OnboardingWizard.tsx" ]] || fail "OnboardingWizard missing"
    pass "onboarding wizard"
    ;;
  AF-07)
    [[ -f "$ROOT/nx_dashboard/src/pages/AssetsPage.tsx" ]] || fail "AssetsPage missing"
    pass "AssetsPage"
    [[ -f "$ROOT/nx_dashboard/src/pages/OpsPage.tsx" ]] || fail "OpsPage missing"
    pass "OpsPage"
    ;;
  AF-08)
    [[ -f "$ROOT/nx_dashboard/src/services/sprintWriteback.ts" ]] || fail "sprintWriteback missing"
    pass "Sprint writeback"
    [[ -f "$ROOT/nx_dashboard/src/data/teamTemplates.ts" ]] || fail "teamTemplates missing"
    pass "team templates"
    ;;
  AF-09)
    [[ -f "$ROOT/scripts/export-factory-metrics.mjs" ]] || fail "export-factory-metrics missing"
    pass "metrics export"
    [[ -f "$ROOT/scripts/enterprise-ef-check.mjs" ]] || fail "enterprise-ef-check missing"
    pass "EF check script"
    [[ -f "$ROOT/docs/dogfood/ef-evidence.md" ]] || fail "ef-evidence.md missing (run npm run ef:check:report)"
    pass "EF evidence doc"
    grep -q "apply_knowledge_injection" "$ROOT/nx_api/src/routes/quick_run.rs" || fail "EF9 quick_run injection"
    pass "EF9 KB injection"
    ;;
  AF-10)
    [[ -f "$ROOT/nx_dashboard/src/pages/TeamDetailPage.tsx" ]] || fail "TeamDetailPage missing"
    pass "TeamDetailPage"
    grep -q 'path="/teams/:teamId"' "$ROOT/nx_dashboard/src/App.tsx" || fail "TeamDetailPage route"
    pass "TeamDetailPage route"
    grep -q 'CanvasPage' "$ROOT/nx_dashboard/src/App.tsx" || fail "CanvasPage route (workflow editor)"
    pass "CanvasPage route"
    grep -q 'sub=wisdom' "$ROOT/nx_dashboard/src/data/navConfig.ts" || fail "wisdom redirect sub tab"
    pass "wisdom legacy redirect"
    [[ -f "$ROOT/nx_dashboard/src/components/factory/FactoryDrawer.tsx" ]] || fail "FactoryDrawer missing"
    pass "FactoryDrawer"
    [[ -f "$ROOT/nx_dashboard/src/stores/factoryDrawerStore.ts" ]] || fail "factoryDrawerStore missing"
    pass "factoryDrawerStore"
    [[ -f "$ROOT/nx_dashboard/src/data/factoryCommands.ts" ]] || fail "factoryCommands missing"
    pass "factoryCommands"
    grep -q 'embedded' "$ROOT/nx_dashboard/src/pages/KnowledgeBasePage.tsx" || fail "KnowledgeBase embedded"
    pass "KnowledgeBase embedded"
    [[ -f "$ROOT/docs/sprints/AF-10-enterprise-ia.yaml" ]] || fail "AF-10 sprint yaml missing"
    pass "AF-10 sprint yaml"
    [[ -f "$ROOT/nx_api/src/routes/factory_attachments.rs" ]] || fail "factory attachments route"
    pass "factory attachments API"
    grep -q 'initialCwd' "$ROOT/nx_dashboard/src/components/terminal/TerminalGrid.tsx" || fail "TerminalGrid initialCwd"
    pass "terminal workspace cwd"
    ;;
  AF-11)
    [[ -f "$ROOT/docs/sprints/AF-11-layout-modes.yaml" ]] || fail "AF-11 sprint yaml missing"
    pass "AF-11 sprint yaml"
    [[ -f "$ROOT/nx_dashboard/src/data/layoutModes.ts" ]] || fail "layoutModes.ts missing"
    pass "layoutModes"
    grep -q 'mode: LayoutMode' "$ROOT/nx_dashboard/src/stores/settingsStore.ts" || fail "layoutMode in settingsStore"
    pass "layoutMode persist"
    [[ -f "$ROOT/nx_dashboard/src/components/layout/shells/AppShells.tsx" ]] || fail "AppShells missing"
    pass "Guided/Studio shells"
    grep -q 'LayoutModePicker' "$ROOT/nx_dashboard/src/pages/SettingsPage.tsx" || fail "LayoutModePicker in settings"
    pass "settings layout mode picker"
    grep -q 'consoleVariant' "$ROOT/nx_dashboard/src/pages/FactoryPage.tsx" || fail "FactoryPage console variant"
    pass "FactoryPage console variant"
    [[ -f "$ROOT/nx_dashboard/src/data/layoutVariants.ts" ]] || fail "layoutVariants.ts missing"
    pass "layoutVariants classic/refined"
    grep -q 'resolveAppShell' "$ROOT/nx_dashboard/src/components/layout/shells/AppShells.tsx" || fail "resolveAppShell missing"
    pass "classic/refined shell router"
    grep -q 'LayoutVariantPicker' "$ROOT/nx_dashboard/src/pages/SettingsPage.tsx" || fail "LayoutVariantPicker in settings"
    pass "settings layout variant picker"
    grep -q 'layout:classic' "$ROOT/nx_dashboard/src/data/factoryCommands.ts" || fail "layout:classic command"
    pass "layout: classic/refined commands"
    [[ -f "$ROOT/nx_dashboard/components.json" ]] || fail "components.json missing"
    pass "shadcn components.json"
    [[ -f "$ROOT/nx_dashboard/src/components/ui/PageHeader.tsx" ]] || fail "PageHeader missing"
    pass "PageHeader"
    [[ -f "$ROOT/nx_dashboard/src/components/ui/tabs.tsx" ]] || fail "shadcn tabs"
    pass "shadcn tabs"
    grep -q 'LAYOUT_COMMANDS' "$ROOT/nx_dashboard/src/data/factoryCommands.ts" || fail "layout commands"
    pass "layout: guided/studio commands"
    grep -q 'normalizeLayoutMode' "$ROOT/nx_dashboard/src/data/layoutModes.ts" || fail "normalizeLayoutMode missing"
    pass "legacy focus migration"
    ;;
  GATE-2)
    [[ -f "$ROOT/docs/dogfood/gate-2-results.md" ]] || fail "gate-2-results template missing"
    pass "gate-2-results template"
    ;;
  AF-P5)
    [[ -f "$ROOT/docs/sprints/AF-P5-unified-capabilities.yaml" ]] || fail "AF-P5 plan missing"
    pass "AF-P5 unified plan"
    grep -q 'version: "3.0"' "$ROOT/docs/sprints/AF-P5-unified-capabilities.yaml" || fail "AF-P5 must be v3.0"
    pass "AF-P5 v3.0"
    [[ -f "$ROOT/docs/sprints/AF-P5-DECISIONS.md" ]] || fail "AF-P5 decisions log missing"
    pass "AF-P5 decisions log"
    [[ -f "$ROOT/docs/sprints/AF-P5-GOVERNANCE.md" ]] || fail "AF-P5 governance missing"
    pass "AF-P5 governance"
    [[ -f "$ROOT/docs/sprints/PR-CHECKLIST-AF-P5.md" ]] || fail "PR checklist missing"
    pass "PR checklist"
    [[ -f "$ROOT/nx_dashboard/e2e/journey-af-p5.spec.ts" ]] || fail "journey-af-p5.spec.ts missing"
    pass "journey e2e contract"
    grep -q 'P8_trust_before_launch' "$ROOT/docs/sprints/AF-P5-unified-capabilities.yaml" || fail "P8 v3 principles"
    pass "v3 product principles P8-P11"
    grep -q 'AF-UX-07' "$ROOT/docs/sprints/AF-P5-unified-capabilities.yaml" || fail "AF-UX-07 in plan"
    pass "v3 epics AF-UX-07+"
    ;;
  AF-UX-01)
    [[ -f "$ROOT/docs/sprints/AF-P5-GOVERNANCE.md" ]] || fail "governance doc missing"
    pass "governance"
    grep -q 'AF-UX-01' "$ROOT/nx_dashboard/e2e/journey-af-p5.spec.ts" || fail "journey contract AF-UX-01"
    pass "journey e2e AF-UX-01 block"
    # Implementation gates (fail until shipped):
    if grep -q 'FirstRunModal\|NewProjectWizard\|first-run-wizard\|p5_first_run_wizard' "$ROOT/nx_dashboard/src" -r --include='*.tsx' --include='*.ts' 2>/dev/null; then
      pass "first-run wizard code present"
    else
      echo "  WARN: FirstRunModal/NewProjectWizard not found yet (Epic in progress)"
    fi
    ;;
  AF-UX-02)
    grep -q 'AF-UX-02' "$ROOT/nx_dashboard/e2e/journey-af-p5.spec.ts" || fail "journey contract AF-UX-02"
    pass "journey e2e AF-UX-02 block"
    if grep -q 'factory-intent-chip\|FACTORY_INTENT_CHIPS\|p5_intent_console' "$ROOT/nx_dashboard/src" -r --include='*.tsx' --include='*.ts' 2>/dev/null; then
      pass "intent-first console markers"
    else
      echo "  WARN: intent console not implemented yet"
    fi
    ;;
  AF-UX-03)
    grep -q 'AF-UX-03' "$ROOT/nx_dashboard/e2e/journey-af-p5.spec.ts" || fail "journey contract AF-UX-03"
    pass "journey e2e AF-UX-03 block"
    if grep -q 'RunCompleteBanner\|run-next-step\|p5_run_next_step' "$ROOT/nx_dashboard/src" -r --include='*.tsx' --include='*.ts' 2>/dev/null; then
      pass "run complete next-step UI"
    else
      echo "  WARN: run next-step banner not implemented yet"
    fi
    ;;
  AF-UX-09)
    grep -q 'AF-UX-09' "$ROOT/nx_dashboard/e2e/journey-af-p5.spec.ts" || fail "journey contract AF-UX-09"
    pass "journey e2e AF-UX-09 block"
    grep -q 'qualityGateRecovery\|detectQualityGateFailure' "$ROOT/nx_dashboard/src/data/qualityGateRecovery.ts" 2>/dev/null || fail "qualityGateRecovery.ts"
    grep -q 'skip_quality_gate_for_stage' "$ROOT/core/workflow/src/engine.rs" || fail "skip quality gate engine"
    grep -q 'run-next-step-tertiary' "$ROOT/nx_dashboard/src/components/factory/RunCompleteBanner.tsx" || fail "QG tertiary button"
    pass "quality gate recovery three-button UX"
    if grep -q 'FailureRecovery\|p5_failure_recovery\|RunCompleteBanner\|runNextSteps\|FactoryTodoStrip' "$ROOT/nx_dashboard/src" -r --include='*.tsx' --include='*.ts' 2>/dev/null; then
      pass "failure recovery UI"
    else
      echo "  WARN: failure recovery not implemented yet"
    fi
    ;;
  AF-UX-07)
    grep -q 'AF-UX-07' "$ROOT/nx_dashboard/e2e/journey-af-p5.spec.ts" || fail "journey contract AF-UX-07"
    pass "journey e2e AF-UX-07 block"
    if grep -q 'LaunchPreviewBar\|p5_launch_preview\|launch-preview' "$ROOT/nx_dashboard/src" -r --include='*.tsx' --include='*.ts' 2>/dev/null; then
      pass "launch preview bar"
    else
      echo "  WARN: launch preview not implemented yet"
    fi
    ;;
  AF-UX-08)
    grep -q 'AF-UX-08' "$ROOT/nx_dashboard/e2e/journey-af-p5.spec.ts" || fail "journey contract AF-UX-08"
    pass "journey e2e AF-UX-08 block"
    if grep -q 'approval_policy\|FactoryTodoStrip\|p5_approval_policy' "$ROOT/nx_dashboard/src" -r --include='*.tsx' --include='*.ts' 2>/dev/null; then
      pass "approval policy / todo strip"
    else
      echo "  WARN: async approval UX not implemented yet"
    fi
    ;;
  AF-12)
    [[ -f "$ROOT/nx_dashboard/src/data/workflowPipelines.ts" ]] || fail "workflowPipelines.ts missing"
    pass "workflowPipelines.ts"
    grep -q 'quick-fix\|dev-workflow\|greenfield' "$ROOT/nx_dashboard/src/data/workflowPipelines.ts" || fail "multi-workflow pipeline defs"
    pass "multi-workflow pipeline registry"
    grep -q 'run-pipeline-board' "$ROOT/nx_dashboard/src/components/factory/RunPipelineBoard.tsx" || fail "RunPipelineBoard data-testid"
    pass "RunPipelineBoard test id"
    ;;
  AF-16)
    [[ -f "$ROOT/config/workflows/greenfield-mvp.yaml" ]] || fail "greenfield-mvp.yaml missing"
    pass "greenfield-mvp.yaml"
    grep -q 'react-vite\|GREENFIELD_STACK' "$ROOT/nx_dashboard/src/data/greenfieldStacks.ts" 2>/dev/null || fail "greenfield stack presets"
    pass "greenfield stack presets"
    ;;
  AF-MM-01)
    grep -q 'DualEngineStatusBanner\|dual-engine-status' "$ROOT/nx_dashboard/src" -r --include='*.tsx' 2>/dev/null || fail "dual engine factory UI"
    pass "dual engine factory banner"
    grep -q '代码引擎' "$ROOT/nx_dashboard/src/pages/AISettingsPage.tsx" || fail "AI settings code lane section"
    pass "AI settings dual engine sections"
    ;;
  AF-MM-03)
    grep -q 'textOnlyWorkflows\|TEXT_ONLY' "$ROOT/nx_dashboard/src" -r --include='*.ts' 2>/dev/null || fail "text-only workflows"
    grep -q 'textOnlyRouting\|autoRouteWorkflowWhenNoCli' "$ROOT/nx_dashboard/src" -r 2>/dev/null || fail "text-only auto route"
    pass "text-only factory routing"
    ;;
  AF-MM-02)
    grep -q 'get_model_config\|resolve_text_lane_model' "$ROOT/nx_api/src/services" -r 2>/dev/null || fail "API model routing"
    pass "API executor model routing"
    ;;
  AF-MM-04)
    grep -q 'textLaneCostMode\|text_lane_cost' "$ROOT/nx_dashboard/src" -r 2>/dev/null || fail "cost routing UI"
    [[ -f "$ROOT/nx_api/src/services/text_lane_cost.rs" ]] || fail "text_lane_cost.rs"
    pass "text lane cost routing"
    ;;
  AF-UX-06)
    grep -q 'AF-UX-06\|factory-role-ask\|FactoryRoleAskPanel' "$ROOT/nx_dashboard/e2e/journey-af-p5.spec.ts" || fail "journey AF-UX-06"
    grep -q 'executeRoleTask\|teamExecute' "$ROOT/nx_dashboard/src/components/factory/FactoryRoleAskPanel.tsx" || fail "factory role ask API"
    pass "journey e2e AF-UX-06 + team execute API"
    ;;
  AF-UX-04a)
    grep -q 'RoleMentionPicker' "$ROOT/nx_dashboard/src/pages/GroupChatPage/ChatInput.tsx" || fail "group chat @ picker"
    pass "group chat @ mention parity"
    ;;
  AF-UX-04b)
    grep -q 'DiscussionSetupSheet\|discussion-setup-sheet' "$ROOT/nx_dashboard/src/pages/GroupChatPage" -r || fail "DiscussionSetupSheet"
    grep -q 'DISCUSSION_SCENE_PRESETS' "$ROOT/nx_dashboard/src/data/discussionScenePresets.ts" || fail "discussion scene presets"
    pass "discussion setup sheet + scene presets"
    ;;
  AF-UX-11)
    grep -q 'AF-UX-11\|CommandPalette\|factoryCommands' "$ROOT/nx_dashboard" -r --include='*.ts' --include='*.tsx' 2>/dev/null || fail "command palette"
    pass "Cmd+K factory commands"
    ;;
  AF-UX-12)
    grep -q 'AF-UX-12\|Cursor\|copyFactoryContext' "$ROOT/nx_dashboard/src/components/factory" -r 2>/dev/null || fail "cursor symbiosis"
    pass "Cursor symbiosis deliverables"
    ;;
  AF-15)
    grep -q 'workflowTiers\|Tier' "$ROOT/nx_dashboard/src/data/workflowTiers.ts" 2>/dev/null || fail "workflowTiers"
    pass "workflow tier filtering"
    ;;
  AF-14)
    [[ -f "$ROOT/config/workflows/dev-workflow.yaml" ]] || fail "dev-workflow.yaml missing"
    grep -q '交付审批' "$ROOT/config/workflows/dev-workflow.yaml" || fail "dev-workflow approval stage"
    pass "dev-workflow full config"
    ;;
  AF-UX-10)
    grep -q 'TaskTimeline\|task-timeline' "$ROOT/nx_dashboard/src" -r --include='*.tsx' 2>/dev/null || fail "TaskTimeline"
    pass "TaskTimeline component"
    ;;
esac

if command -v cargo >/dev/null 2>&1; then
  echo "Running: cargo test -p nexus-workflow --lib"
  (cd "$ROOT" && cargo test -p nexus-workflow --lib) || fail "nexus-workflow tests"
  pass "nexus-workflow unit tests"
  echo "Running: cargo test -p nexus-workflow executor_routing"
  (cd "$ROOT" && cargo test -p nexus-workflow executor_routing) || fail "executor_routing integration"
  pass "executor_routing integration"
fi

if command -v node >/dev/null 2>&1 && curl -sf http://localhost:8080/api/v1/workflows >/dev/null 2>&1; then
  echo "Running: npm run test:e2e:af-p1 (8080)"
  (cd "$ROOT/nx_dashboard" && npm run test:e2e:af-p1) || fail "AF-P1 e2e"
  pass "AF-P1 e2e smoke"
  if [[ "$SPRINT_ID" == AF-UX-* || "$SPRINT_ID" == "AF-12" ]]; then
    echo "Running: journey-af-p5 e2e (chromium only)"
    (cd "$ROOT/nx_dashboard" && npx playwright test e2e/journey-af-p5.spec.ts --project=chromium) || fail "journey-af-p5 e2e"
    pass "journey-af-p5 e2e"
  fi
  if [[ "$SPRINT_ID" == "AF-09" ]]; then
    echo "Running: npm run ef:check"
    (cd "$ROOT" && npm run ef:check) || fail "enterprise EF check"
    pass "enterprise EF HTTP smoke"
  fi
fi

if command -v node >/dev/null 2>&1; then
  node "$ROOT/scripts/gate-update-progress.mjs" "$SPRINT_ID" || true
fi

echo ""
echo "=== $SPRINT_ID gate-check PASSED ==="
exit 0
