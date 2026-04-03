# Bootstrap Module

Design Reference: `docs/03-module-design/core/bootstrap.md`

This module implements the 8-stage initialization system:

1. Stage 1: Infrastructure (logging_system)
2. Stage 2: Security and Storage (security_manager, storage_service)
3. Stage 3: Basic Services and Event (llm_provider, tool_system, event_loop, timer_system)
4. Stage 4: Core Engine Layer (hook_engine, session_manager, router, monitor)
5. Stage 5: Agent Layer (agent_variants, agent_runtime, external_agent, skill_engine, orchestrator, task_manager, command, workflows_directory)
6. Stage 6: Report (report_skill)
7. Stage 7: Context Compression (context_compressor)
8. Stage 8: Security Layer (sandbox, ipc_contract)
