# Lexia Context

This glossary defines the product-specific language used by Lexia and its agents. Use these terms consistently in code, tickets, documentation, and prompts.

## Product and runtime

**Lexia**:
The user-facing agent harness and the agent identity that coordinates routing, execution, tools, skills, workflows, and persistent context.
_Avoid_: Alexia, Lexia-Agent when referring to the product itself.

**EVE runtime**:
The server-side execution runtime that runs an agent turn, tools, workflows, skills, and model calls after routing.
_Avoid_: frontend agent, client agent.

**Jev gate**:
The typed decision stage that classifies a request before execution and may select a builtin action, tool/workflow, or model route.
_Avoid_: Jev agent, Jev chatbot.

**model tier**:
A logical capability class selected by Jev, such as `small`, `general`, `complex`, or `deep`; provider-specific model IDs belong in configuration.
_Avoid_: hard-coded model name when discussing routing policy.

## Installed runtime

**EVE release**:
An immutable, versioned directory containing an EVE source snapshot and its generated production runtime output.
_Avoid_: live agent folder, mutable production agent.

**active release**:
The EVE release currently selected by Lexia to serve user requests.
_Avoid_: latest release; a staged release is not active until promoted.

**staged release**:
An imported and validated EVE release that has not yet been promoted to serve traffic.
_Avoid_: update branch, temporary worktree.

**promotion**:
The controlled transition that health-checks a staged release, makes it active, and restarts the EVE process.
_Avoid_: hot patch.

## Work execution

**workflow**:
A documented, repeatable multi-step operation with scripts stored under a named directory in `workflow/`.
_Avoid_: ad hoc script, command bundle.

**skill**:
An instruction and capability package that teaches an agent how to perform a bounded class of work.
_Avoid_: model, workflow.

**agent task**:
A user-approved unit of coding, research, or operational work that may run in an isolated Git worktree.
_Avoid_: background job when user review and promotion are required.
