---
status: accepted
---

# Keep EVE releases external to the Lexia binary

Lexia will be distributed as a stable Bun-compiled host containing SvelteKit and the release supervisor, while EVE will be built and run from external, versioned release folders. User updates are staged, validated, health-checked, promoted, and restartable without recompiling the Lexia executable; previous releases remain available for local rollback. This preserves a small stable client/server host while allowing the agent runtime to evolve independently.
