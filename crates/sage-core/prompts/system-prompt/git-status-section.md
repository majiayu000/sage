---
name: git-status-section
description: Git status information (conditional)
version: "1.0.0"
category: system-prompt
variables:
  - IS_GIT_REPO
  - GIT_BRANCH
  - MAIN_BRANCH
---

${IS_GIT_REPO?`gitStatus: This is the git status at the start of the conversation. Note that this status is a snapshot in time, and will not update during the conversation.
Current branch: ${GIT_BRANCH}

Main branch (you will usually use this for PRs): ${MAIN_BRANCH}
`:``}
