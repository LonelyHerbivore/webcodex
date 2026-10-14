import test from "node:test";
import assert from "node:assert/strict";
import { app, flush, toolResult } from "./app_test_support.mjs";

const waitId = "wc_job_wait_q6urq6urq6urq6ur";
const jobId = "wc_job_job-terminal-harness";
const attemptId = "wc_job_delivery_ZmZmZmZmZmZmZmZm";
const farFuture = 4_102_444_800;
const input = { wait_id: waitId };
const waiting = {
  version: 1,
  wait_id: waitId,
  job_id: jobId,
  state: "waiting",
  delivery_state: "not_ready",
  terminal_status: null,
  terminal_outcome: null,
  automatic_resume_available: true,
  expires_at: farFuture,
  fallback_tool: "observe_jobs",
};
const pending = {
  ...waiting,
  state: "triggered",
  delivery_state: "pending",
  terminal_status: "completed",
  terminal_outcome: "succeeded",
};
const preparedProjection = { ...pending, delivery_state: "prepared" };

const calls = (view, name) => view.calls(name);
const hostMessages = view => view.sent.filter(message => message.method === "ui/message");
const bindingId = view => calls(view, "bind_job_terminal_continuation")[0].params.arguments.binding_id;
const plain = value => JSON.parse(JSON.stringify(value));

const bindResult = projection => toolResult({
  job_terminal_continuation: projection,
  host_binding: { bound: true },
  state_changed: true,
});
const stateResult = (projection, preparedAttemptId = null) => toolResult({
  job_terminal_continuation: projection,
  host_binding: { bound: true },
  app_protocol: { prepared_attempt_id: preparedAttemptId },
});
const prepareResult = (message = "Continue exact Job terminal test") => toolResult({
  wait_id: waitId,
  job_id: jobId,
  delivery_state: "prepared",
  attempt_id: attemptId,
  dispatch_observation: "dispatch_prepared",
  state_changed: true,
  app_protocol: { automatic_message: message },
});
const finishResult = deliveryState => toolResult({
  wait_id: waitId,
  job_id: jobId,
  attempt_id: attemptId,
  delivery_state: deliveryState,
  dispatch_observation: deliveryState === "delivered" ? "dispatch_accepted" : "delivery_unknown",
  state_changed: true,
});

async function initializedView({ initialProjection = waiting, yieldCurrentTurn = true } = {}) {
  const view = app("mcp_job_terminal_continuation_app.html");
  view.toolInput(input);
  await view.initialize();
  view.toolResult({ job_terminal_continuation: initialProjection });
  await flush();
  if (yieldCurrentTurn) view.advanceTime(10000);
  assert.equal(calls(view, "bind_job_terminal_continuation").length, 1);
  assert.match(bindingId(view), /^wc_host_binding_[A-Za-z0-9_-]{21}[AQgw]$/);
  assert.deepEqual(plain(calls(view, "bind_job_terminal_continuation")[0].params.arguments), {
    wait_id: waitId,
    binding_id: bindingId(view),
  });
  return view;
}

async function boundWaitingView(options) {
  const view = await initializedView(options);
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(waiting));
  assert.equal(calls(view, "get_job_terminal_continuation_state").length, 1);
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(waiting));
  return view;
}

test("terminal discovered before the invoking turn yield grace cannot dispatch early", async () => {
  const view = await boundWaitingView({ yieldCurrentTurn: false });
  await view.fireTimers(3000);
  let terminalState = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(terminalState, stateResult(pending));

  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 0);
  assert.equal(hostMessages(view).length, 0);
  assert.ok([...view.timers.values()].some(timer => timer.delay === 7000));

  await view.fireTimers(7000);
  terminalState = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(terminalState, stateResult(pending));
  const prepare = calls(view, "prepare_job_terminal_continuation").at(-1);
  assert.ok(prepare, "dispatch becomes eligible only after the bounded turn-yield grace");
  await view.reply(prepare, prepareResult());
  assert.equal(hostMessages(view).length, 1);
});

test("already-triggered presentation result suppresses redundant automatic follow-up", async () => {
  const view = await initializedView({ initialProjection: pending, yieldCurrentTurn: false });
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 0);
  assert.equal(hostMessages(view).length, 0);
  assert.equal(view.timers.size, 0);
});

test("Job terminal App binds before terminal, discovers completion without observe_jobs, and dispatches once", async () => {
  const view = await boundWaitingView();
  assert.equal(hostMessages(view).length, 0);
  assert.equal(calls(view, "observe_jobs").length, 0);

  await view.fireTimers(3000);
  const terminalState = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(terminalState, stateResult(pending));
  const prepare = calls(view, "prepare_job_terminal_continuation").at(-1);
  assert.ok(prepare);
  assert.deepEqual(plain(prepare.params.arguments), { wait_id: waitId, binding_id: bindingId(view) });
  await view.reply(prepare, prepareResult());

  assert.equal(hostMessages(view).length, 1);
  assert.deepEqual(plain(hostMessages(view)[0].params), {
    role: "user",
    content: [{ type: "text", text: "Continue exact Job terminal test" }],
  });
  await view.reply(hostMessages(view)[0], {});
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  assert.ok(finish);
  assert.deepEqual(plain(finish.params.arguments), {
    wait_id: waitId,
    binding_id: bindingId(view),
    attempt_id: attemptId,
    outcome: "dispatch_accepted",
  });
  await view.reply(finish, finishResult("delivered"));

  assert.equal(hostMessages(view).length, 1);
  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 1);
  assert.equal(calls(view, "finish_job_terminal_continuation").length, 1);
  assert.equal(calls(view, "observe_jobs").length, 0);
  assert.equal(view.timers.size, 0, "delivered carrier stops polling");
  await view.visibility(true);
  await view.visibility(false);
  assert.equal(hostMessages(view).length, 1, "visibility changes cannot redispatch");
});

test("Host ui/message rejection records delivery_unknown and never retries dispatch", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  await view.reply(calls(view, "prepare_job_terminal_continuation")[0], prepareResult());
  assert.equal(hostMessages(view).length, 1);
  await view.reject(hostMessages(view)[0]);
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  assert.equal(finish.params.arguments.outcome, "delivery_unknown");
  await view.reply(finish, finishResult("delivery_unknown"));
  assert.equal(hostMessages(view).length, 1);
  assert.equal(view.timers.size, 0);
});

test("finish response loss retries only the exact finish fence, never ui/message", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  await view.reply(calls(view, "prepare_job_terminal_continuation")[0], prepareResult());
  await view.reply(hostMessages(view)[0], {});
  const firstFinish = calls(view, "finish_job_terminal_continuation")[0];
  await view.reject(firstFinish);
  assert.equal(hostMessages(view).length, 1);
  await view.fireTimers(3000);
  const finishes = calls(view, "finish_job_terminal_continuation");
  assert.equal(finishes.length, 2);
  assert.deepEqual(plain(finishes[1].params.arguments), plain(firstFinish.params.arguments));
  await view.reply(finishes[1], finishResult("delivered"));
  assert.equal(hostMessages(view).length, 1);
  assert.equal(view.timers.size, 0);
});

test("finish response loss across Server restart rebinds only to observe recovered unknown", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  await view.reply(calls(view, "prepare_job_terminal_continuation")[0], prepareResult());
  await view.reply(hostMessages(view)[0], {});
  const firstFinish = calls(view, "finish_job_terminal_continuation")[0];
  await view.reject(firstFinish);
  assert.equal(hostMessages(view).length, 1);

  await view.fireTimers(3000);
  const secondFinish = calls(view, "finish_job_terminal_continuation")[1];
  assert.ok(secondFinish);
  await view.reject(secondFinish);
  const staleState = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reject(staleState);

  const rebind = calls(view, "bind_job_terminal_continuation")[1];
  assert.ok(rebind);
  assert.equal(
    rebind.params.arguments.binding_id,
    bindingId(view),
    "restart recovery reuses only the surviving View fence",
  );
  await view.reply(rebind, bindResult({ ...pending, delivery_state: "delivery_unknown" }));

  assert.equal(hostMessages(view).length, 1, "restart recovery never resends ui/message");
  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 1);
  assert.equal(view.timers.size, 0);
});

test("malformed prepare response is reconciled as unknown after authoritative prepared state", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  const prepare = calls(view, "prepare_job_terminal_continuation")[0];
  await view.reply(prepare, toolResult({
    wait_id: waitId,
    job_id: jobId,
    delivery_state: "prepared",
    attempt_id: attemptId,
    dispatch_observation: "dispatch_prepared",
    state_changed: true,
  }));
  assert.equal(hostMessages(view).length, 0, "malformed post-fence response must not dispatch");
  await view.fireTimers(3000);
  const state = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(state, stateResult(preparedProjection, attemptId));
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  assert.equal(finish.params.arguments.outcome, "delivery_unknown");
  await view.reply(finish, finishResult("delivery_unknown"));
  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 1);
  assert.equal(hostMessages(view).length, 0);
});

test("prepare response loss reconciles authoritative prepared state without Host redispatch", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  const prepare = calls(view, "prepare_job_terminal_continuation")[0];
  await view.reject(prepare);
  assert.equal(hostMessages(view).length, 0);
  await view.fireTimers(3000);
  const state = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(state, stateResult(preparedProjection, attemptId));
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  assert.equal(finish.params.arguments.outcome, "delivery_unknown");
  await view.reply(finish, finishResult("delivery_unknown"));
  assert.match(view.nodes.status.textContent, /Prepare result uncertain; Host dispatch not confirmed/);
  assert.equal(hostMessages(view).length, 0, "uncertain prepare never sends ui/message");
});

test("prepared state not dispatched by this View is finalized unknown without ui/message", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(preparedProjection));
  await view.reply(
    calls(view, "get_job_terminal_continuation_state")[0],
    stateResult(preparedProjection, attemptId),
  );
  assert.equal(hostMessages(view).length, 0);
  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 0);
  const finish = calls(view, "finish_job_terminal_continuation")[0];
  assert.equal(finish.params.arguments.outcome, "delivery_unknown");
  await view.reply(finish, finishResult("delivery_unknown"));
  assert.match(view.nodes.status.textContent, /Prepared attempt not dispatched by this View/);
  assert.equal(hostMessages(view).length, 0, "unowned prepared fence cannot redispatch");
});

test("teardown before prepare unbinds and leaves no Host dispatch", async () => {
  const view = await boundWaitingView();
  await view.teardown();
  assert.equal(calls(view, "unbind_job_terminal_continuation").length, 1);
  assert.equal(hostMessages(view).length, 0);
});

test("teardown after prepare never creates a second Host dispatch", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  await view.reply(calls(view, "prepare_job_terminal_continuation")[0], prepareResult());
  assert.equal(hostMessages(view).length, 1);
  await view.teardown();
  assert.equal(calls(view, "unbind_job_terminal_continuation").length, 1);
  assert.equal(hostMessages(view).length, 1);
});

test("process binding loss performs a bounded rebind and may recover pending delivery", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(waiting));
  const firstState = calls(view, "get_job_terminal_continuation_state")[0];
  await view.reject(firstState);
  assert.equal(calls(view, "bind_job_terminal_continuation").length, 2);
  assert.equal(
    calls(view, "bind_job_terminal_continuation")[1].params.arguments.binding_id,
    bindingId(view),
    "the same View fence is reused after process-local binding loss",
  );
  await view.reply(calls(view, "bind_job_terminal_continuation")[1], bindResult(pending));
  const recoveredState = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(recoveredState, stateResult(pending));
  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 1);
});

test("expired projection stops terminal polling", async () => {
  const view = await initializedView();
  const expired = { ...pending, expires_at: 1 };
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(expired));
  assert.equal(calls(view, "get_job_terminal_continuation_state").length, 0);
  assert.equal(view.timers.size, 0);
  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 0);
  assert.equal(hostMessages(view).length, 0);
});

test("hidden visibility uses bounded deterministic backoff and visible reset", async () => {
  const view = await boundWaitingView();
  await view.visibility(true);
  let state = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(state, stateResult(waiting));
  assert.ok([...view.timers.values()].some(timer => timer.delay === 15000));

  for (let index = 0; index < 20; index += 1) {
    await view.fireTimers(15000);
    state = calls(view, "get_job_terminal_continuation_state").at(-1);
    await view.reply(state, stateResult(waiting));
  }
  assert.ok([...view.timers.values()].some(timer => timer.delay === 60000));

  for (let index = 0; index < 25; index += 1) {
    await view.fireTimers(60000);
    state = calls(view, "get_job_terminal_continuation_state").at(-1);
    await view.reply(state, stateResult(waiting));
  }
  assert.ok([...view.timers.values()].some(timer => timer.delay === 300000));

  await view.visibility(false);
  state = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(state, stateResult(waiting));
  assert.ok([...view.timers.values()].some(timer => timer.delay === 3000));

  await view.visibility(true);
  state = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(state, stateResult(waiting));
  assert.ok([...view.timers.values()].some(timer => timer.delay === 15000));
});

test("Host bridge timeout labels the local uncertainty without creating retry authority", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  await view.reply(calls(view, "prepare_job_terminal_continuation")[0], prepareResult());
  assert.equal(hostMessages(view).length, 1);
  await view.fireTimers(10000);
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  assert.equal(finish.params.arguments.outcome, "delivery_unknown");
  await view.reply(finish, finishResult("delivery_unknown"));
  assert.match(view.nodes.status.textContent, /Host bridge timed out after 10 seconds/);
  assert.equal(hostMessages(view).length, 1, "timeout never licenses second ui/message");
});

test("Host ui/message explicit bridge error is distinguished without exposing raw host details", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  await view.reply(calls(view, "prepare_job_terminal_continuation")[0], prepareResult());
  await view.reject(hostMessages(view)[0], { code: -32000, message: "sensitive private Host exception" });
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  assert.equal(finish.params.arguments.outcome, "delivery_unknown");
  await view.reply(finish, finishResult("delivery_unknown"));
  assert.match(view.nodes.status.textContent, /Host returned a bridge error/);
  assert.doesNotMatch(view.nodes.status.textContent, /sensitive private/);
  assert.equal(hostMessages(view).length, 1);
});

test("late Host ui/message reply after 10-second timeout cannot create a second outcome", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  await view.reply(calls(view, "prepare_job_terminal_continuation")[0], prepareResult());
  const original = hostMessages(view)[0];
  assert.ok(original, "one original Host message exists");
  await view.fireTimers(10000);
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  assert.equal(finish.params.arguments.outcome, "delivery_unknown");
  await view.reply(original, {}, "Host reply delivered after the local bridge timeout");
  await view.reply(finish, finishResult("delivery_unknown"));
  await view.fireTimers(3000);
  assert.equal(hostMessages(view).length, 1, "late Host response cannot authorize redispatch");
  assert.equal(calls(view, "finish_job_terminal_continuation").length, 1);
  assert.match(view.nodes.status.textContent, /Host bridge timed out after 10 seconds/);
});

test("background visibility transition during ambiguous Host delivery never redispatches", async () => {
  const view = await initializedView();
  await view.reply(calls(view, "bind_job_terminal_continuation")[0], bindResult(pending));
  await view.reply(calls(view, "get_job_terminal_continuation_state")[0], stateResult(pending));
  await view.reply(calls(view, "prepare_job_terminal_continuation")[0], prepareResult());
  assert.equal(hostMessages(view).length, 1);
  await view.visibility(true);
  await view.fireTimers(10000);
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  assert.equal(finish.params.arguments.outcome, "delivery_unknown");
  await view.reply(finish, finishResult("delivery_unknown"));
  await view.visibility(false);
  await view.fireTimers(3000);
  assert.equal(hostMessages(view).length, 1, "visibility cannot resend an uncertain Host message");
  assert.equal(calls(view, "prepare_job_terminal_continuation").length, 1);
});

test("Host delivered with a 2608-second lag reports delay but never claims a new model turn", async () => {
  const view = await boundWaitingView();
  await view.fireTimers(3000);
  const poll = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(poll, stateResult({
    ...pending,
    delivery_state: "delivered",
    terminal_to_delivery_settlement_seconds: 2608,
  }));
  assert.match(view.nodes.status.textContent, /43m 28s after Job/);
  assert.match(view.nodes.status.textContent, /new ChatGPT turn unverified/);
  assert.equal(hostMessages(view).length, 0);
  assert.equal(view.timers.size, 0);
});

test("malformed and unsettled latency never appears as a settlement fact", async () => {
  for (const invalid of [-1, "2608", Number.POSITIVE_INFINITY, 58]) {
    const view = await boundWaitingView();
    await view.fireTimers(3000);
    const poll = calls(view, "get_job_terminal_continuation_state").at(-1);
    await view.reply(poll, stateResult({
      ...pending,
      delivery_state: "delivered",
      terminal_to_delivery_settlement_seconds: invalid,
    }));
    assert.doesNotMatch(view.nodes.status.textContent, /settled/);
    assert.match(view.nodes.status.textContent, /new ChatGPT turn unverified/);
    assert.equal(hostMessages(view).length, 0);
  }
});

test("frozen 2608-second background cannot dispatch until visibility resumes (diagnostic reproduction)", async () => {
  const view = await boundWaitingView();
  await view.visibility(true);
  const hiddenPoll = calls(view, "get_job_terminal_continuation_state").at(-1);
  await view.reply(hiddenPoll, stateResult(waiting));
  const expectedHiddenTimer = [...view.timers.values()];
  assert.ok(expectedHiddenTimer.some(timer => timer.delay === 15000));
  const before = calls(view, "get_job_terminal_continuation_state").length;
  view.advanceTime(2608000); // Simulates a suspended iframe: no timer callbacks.
  assert.equal(calls(view, "get_job_terminal_continuation_state").length, before);
  assert.equal(hostMessages(view).length, 0);

  await view.visibility(false); // Browser tab becomes visible again.
  const resumedPoll = calls(view, "get_job_terminal_continuation_state").at(-1);
  assert.equal(calls(view, "get_job_terminal_continuation_state").length, before + 1);
  await view.reply(resumedPoll, stateResult(pending));
  const prepare = calls(view, "prepare_job_terminal_continuation").at(-1);
  assert.ok(prepare, "foreground transition can recover a pending Wait");
  await view.reply(prepare, prepareResult());
  assert.equal(hostMessages(view).length, 1, "exactly one Host dispatch attempt");
  await view.reply(hostMessages(view)[0], {});
  const finish = calls(view, "finish_job_terminal_continuation").at(-1);
  await view.reply(finish, finishResult("delivered"));
  assert.equal(hostMessages(view).length, 1);
});
