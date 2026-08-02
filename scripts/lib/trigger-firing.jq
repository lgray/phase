# CR 603.7 firing carriers for a persisted game dump (upstream #6842, 8121fd1c6).
#
# SINGLE DEFINITION of the derivation. Both the pristine regeneration path and
# the in-place stamping path load THIS file, so neither can certify its own copy
# of the recipe.
#
# #6842 made a `TriggerFiring` carrier MANDATORY on every persisted triggered
# record and fails CLOSED without one, so a dump captured before that commit
# cannot load at all. The read-only pristine root predates it too (captured
# 2026-07-22/25), so the value cannot be recovered by re-reading the dump — it
# must be DERIVED, per record.
#
# `TriggerFiring::UnknownLegacy` is NOT an escape hatch: `validate_firing`
# rejects it for a live carrier ("has no canonical trigger firing
# discriminator") because it is the field-absent marker (`skip_serializing_if`)
# and the redaction default, never a legal persisted value.
#
# THE DISCRIMINANT, CR 603.1 vs CR 603.7a:
#   ORDINARY <= the fired trigger's definition is present on its SOURCE OBJECT's
#               own `trigger_definitions` / `base_trigger_definitions`, matched
#               by exact `description`. A printed or granted triggered ability of
#               a permanent is an ordinary triggered ability.
#   DELAYED  <= the trigger has an install receipt in `delayed_triggers`. Every
#               dump in this corpus records `delayed_triggers: []` and no install
#               journal, so `Delayed(Some(..))` could not validate regardless —
#               `validate_firing` demands a registered install root.
# Anything else ABORTS by name. There is deliberately no fallback stamp: a wrong
# carrier silently re-classifies a CR 603.7 firing identity, which is exactly the
# inference upstream refuses to make.
#
# `stack_trigger_firings` is keyed by the STACK ENTRY id — what
# `validate_trigger_firing_coherence` looks up — not by the source id.

def _defs($objs; $src):
  (($objs[($src|tostring)] // {})
   | (.trigger_definitions // []) + (.base_trigger_definitions // []))
  | map(.description // "");

def _firing($objs; $src; $d):
  if ((_defs($objs; $src)) | index($d)) then "Ordinary"
  else error("UNDETERMINED firing carrier: source=\($src) description=\($d)")
  end;

# How many carriers this dump actually needs. 0 means the dump records no
# triggered pending/stack/resolving entry at all, so stamping it is a NO-OP and
# any "the bytes changed" control arm over it would be reporting jq
# re-serialization, not a stamp.
def trigger_carrier_count:
  (if ((.gameState.pending_trigger // null) != null) then 1 else 0 end)
  + ([ (.gameState.stack // [])[] | select(.kind.type == "TriggeredAbility") ] | length)
  + (if (((.gameState.resolving_stack_entry // .gameState.resolving_trigger).kind.type? // "")
         == "TriggeredAbility") then 1 else 0 end);

# Pass a dump that is not `gameState`-shaped straight through. Several fixtures
# in this corpus are stored in a different envelope (top level `turn_number`,
# not `gameState`); without this guard `.gameState |= ...` would CREATE a
# gameState key on them, i.e. corrupt them.
# CR 603.7 delayed-trigger ALLOCATORS — a second field class #6842 repairs at
# load time, and only on ONE of the two decode paths.
#
# `next_delayed_trigger_token` carries `#[serde(default)]`, so a pre-#6842 dump
# that omits it restores as 0 through a bare `GameState` decode. The production
# `PersistedGameState` path instead runs the load-time migration
#   next = max(existing // 1, (max used token) + 1)
# and restores 1. The two paths therefore disagree on a legacy dump, and 0 is
# invalid on its face: `validate_trigger_firing_coherence` rejects
# `next_delayed_trigger_token <= max_token`, and `max_token` is 0 when there are
# no install roots. Stamping the repaired value on disk makes the fixture look
# like a modern capture, which keeps the two decoders in agreement WITHOUT
# relaxing the assertion, and survives the eventual deletion of the shim.
#
# The GENERAL derivation of the used-token set is ENGINE logic — it walks
# `resolved_rules_journal` install commands and `delayed_triggers` provenance,
# with reuse and nonzero checks. Re-deriving that here would repeat exactly the
# mistake `migrate-dump-fixture.sh` refuses to make for `EffectKind`. So this
# stamps ONLY the case where the formula collapses to a constant — no install
# roots at all, so both used sets are empty and the result is
# `max(existing // 1, 1)` — and ABORTS BY NAME otherwise, leaving the general
# case to the engine.
def stamp_delayed_allocators:
  if (.gameState // null) == null then . else
    ([ (.gameState.resolved_rules_journal.entries // [])[]
       | select(.command.DelayedTriggerInstall) ] | length) as $installs
  | ((.gameState.delayed_triggers // []) | length) as $delayed
  | if ($installs > 0 or $delayed > 0)
    then error("UNDETERMINED delayed-trigger allocators: \($installs) install command(s), \($delayed) delayed trigger(s) — deriving the used-token set is engine logic, not jq's")
    else .gameState.next_delayed_trigger_token
           = ([(.gameState.next_delayed_trigger_token // 1), 1] | max)
       | .gameState.next_delayed_trigger_instance
           = ([(.gameState.next_delayed_trigger_instance // 1), 1] | max)
    end
  end;

def stamp_trigger_firing:
  if (.gameState // null) == null then . else
  .gameState.objects as $objs
  | (.gameState.resolving_stack_entry // .gameState.resolving_trigger) as $rt
  | .gameState |= (
      (if (.pending_trigger // null) != null
         then .pending_trigger_firing =
                _firing($objs; .pending_trigger.source_id; .pending_trigger.description)
         else . end)
    | (([ (.stack // [])[]
          | select(.kind.type == "TriggeredAbility")
          | {key: (.id|tostring),
             value: _firing($objs; .kind.data.source_id; .kind.data.description)} ]
        | from_entries) as $sf
       | if ($sf | length) > 0 then .stack_trigger_firings = $sf else . end)
    | (if ($rt != null and ($rt.kind.type? // "") == "TriggeredAbility")
         then .resolving_trigger_firing =
                _firing($objs; $rt.kind.data.source_id; $rt.kind.data.description)
         else . end)
    )
  end;
