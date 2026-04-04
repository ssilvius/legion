#!/bin/bash
# Legion Stop hook: prompt the agent to reflect before closing
# Mutex: only fires once per session AND only if the session did real work.
INPUT=$(cat)
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')

if [ -z "$CWD" ]; then
  exit 0
fi

REPO=$(basename "$CWD")
CWD_HASH=$(echo "$CWD" | md5 -q 2>/dev/null || echo "$CWD" | md5sum 2>/dev/null | cut -d' ' -f1)

# Prevent re-fires: one reflect prompt per session
MARKER="/tmp/legion-reflected-${CWD_HASH}"
if [ -f "$MARKER" ]; then
  exit 0
fi

# Mutex: skip if session had no real work (no work marker from recall-first or other hooks)
WORK_MARKER="/tmp/legion-work-${CWD_HASH}"
if [ ! -f "$WORK_MARKER" ]; then
  exit 0
fi

# Create marker. SessionStart hook cleans these up on next session.
touch "$MARKER"

# Check for unread bullpen posts
BOARD_COUNT=$(legion bullpen --count --repo "$REPO" 2>/dev/null)
BOARD_MSG=""
if [ -n "$BOARD_COUNT" ]; then
  BOARD_MSG="

(3) You have unread board posts: ${BOARD_COUNT}. Run legion bullpen --repo ${REPO} to read and respond before leaving."
fi

jq -n --arg reason "Before you stop:

(1) TEAM FIRST: Did you help a teammate this session? Did you respond to board posts directed at you? If someone asked for help and you ignored it, that is the thing to fix next session.

(2) REFLECT: What would you tell another agent who hits this same problem tomorrow? Store it: legion reflect --repo $REPO --text '<your reflection here>'

(3) BOOST what helped: legion boost --id <id>. SIGNAL what is unresolved: legion signal --repo $REPO --to <agent> --verb question --note '<question>'${BOARD_MSG}" '{
  "decision": "block",
  "reason": $reason
}'
