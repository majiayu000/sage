---
name: environment-info
description: Environment information template
version: "1.0.0"
category: system-prompt
variables:
  - WORKING_DIR
  - IS_GIT_REPO
  - PLATFORM
  - CURRENT_DATE
---

Here is useful information about the environment you are running in:
<env>
Working directory: ${WORKING_DIR}
Is directory a git repo: ${IS_GIT_REPO?`Yes`:`No`}
Platform: ${PLATFORM}
Today's date: ${CURRENT_DATE}
</env>
