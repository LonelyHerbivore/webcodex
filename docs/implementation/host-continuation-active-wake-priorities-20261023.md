# Host continuation: function-first priority (2026-10-23)

**Directive:** Build and verify normal-network GPT automatic continuation before any network fluctuation, reconnection or offline fault-injection work.

## P0 — Make active ChatGPT continuation work end-to-end

1. Re-establish an isolated, exact-source test Host using PR #6 HEAD 8d37dd3e2853806e2c4e38af45d7076f4c425aeb. Both the main CI and Windows trio build passed, but that build has not been verified or deployed on the local machine. The test connector is currently disconnected. Its latest uncertain Tunnel run fence requires separate exact-run recovery approval; preserve it until then. Do not touch production.
2. While the ChatGPT tab is **visible** and the network is healthy, trigger one authorized disposable Agent completion / work_done. Allow the original model turn to end. Verify the App requests automatic continuation without any manual user message.
3. Demand **real** follow-through: a fresh GPT model turn, exact consume_agent_wake for the triggering Wake, result readback and the next Agent/task instruction. Record user actions = 0, no duplicate dispatch/Worker start and independent Host/model evidence where available. A UI message acknowledgement, green unit tests or a Shell output marker alone do not count.
4. Confirm a second sequential Agent completion in the same normal workflow. Then verify the ordinary visible-tab Job Terminal route using a permitted stimulus; do not retry the ChatGPT-blocked run_job operation via FastCtx or another route.

**P0 complete:** real test ChatGPT auto-continues after Agent work_done and actually assigns the next authorized task, without the user typing “继续”; exact Wake consumed once. Record separately whether a reliable independent model Turn ID was accessible. Do not mark the full release gate passed from an observed UI effect alone.

## P1 — Normal-network workflow and background capability

- Integrate proven automatic progression into the real Orca Coordinator workflow (Agent Team, terminal reports, queued tasks). Use isolated rehearsal before any changes to the user's live novel workspace.
- Verify the separate Job completion carrier, repeated completions and duplicate prevention.
- Investigate separately authorized Host-native background/closed-tab turn scheduling. The existing MCP App currently suppresses automatic UI messages when hidden; foreground resume must not be presented as true background wake.

## P2 — Network fluctuations and recovery (deferred)

- WAN outage, Tunnel disconnect/reconnect, delivery uncertainty on transport loss, Desktop restart, failover and replay reconciliation.
- Keep existing network-recovery fixes and tests intact, but do not spend the current sprint adding outage-only code or fault injection before P0 genuine auto-wake works.

## Test boundaries and current evidence

The prior six-case release matrix remains a **historical/full-release audit**. For this narrower P0, visible Agent completion comes first; background and network failure do not block demonstration of the basic feature, although the full release remains gated separately. Current PR #6 is Draft. Source-level Agent yield fix and CI are green; **zero genuine PR #6 ChatGPT fresh-turn cases have passed**. The old local c2d19dc9 build/connection cannot substitute for testing 8d37dd3. Never fabricate Host events, silently delete Tunnel run fences, or use another tool to evade an explicit platform safety block.

**Next actual step:** once the latest Test run fence has been safely resolved with exact-run approval, verify the isolated new binary and Test connector, then run the visible Agent work_done -> GPT fresh turn -> exact Wake consumed -> next-task assignment chain.
