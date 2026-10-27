# Check host resources before starting an agent

`board.agents` limits how many tasks run together. A spare slot does not mean
that the host still has memory or CPU capacity when its agent starts. An optional
resource probe adds a fresh check at queue claims, actual program starts and
prompts that wake existing task sessions. It also gates reviewer program starts;
a refused reviewer start follows the existing retryable review-error path rather
than starting a model under pressure.
The Coordinator has an explicit, scoped exemption in its own caller. Programs
started by hand outside the board are outside this task gate. With the gate
enabled, an unknown or ambiguous task-pane binding holds instead of bypassing
the resource check.

Enable it with an absolute executable path, not a shell command:

```sh
herdr-projects set board.admission /usr/local/bin/herdr-host-reading
```

The default is `none`, which preserves slot-only admission. The command is your
board setting, never a repository's setting. It receives no arguments and must
only read host telemetry. It writes one JSON object to stdout and exits:

```json
{
  "schema": 1,
  "sampled_at": 1793000000000,
  "total_bytes": 42949672960,
  "available_bytes": 12884901888,
  "cores": 14,
  "load1": 8.0,
  "load5": 8.0,
  "pressure": 2,
  "free_percent": 45.0,
  "swap_growth_bytes_per_second": 0.0,
  "swapout_bytes_per_second": 0.0
}
```

`sampled_at` is Unix time in milliseconds, after collecting the reading, not a
cached file's modification time. Load is the one- and five-minute load average,
not a percentage. Available bytes are the host's conservative reclaimable-memory
estimate; `free_percent` is its pressure monitor's separate free-memory estimate.
Pressure uses 1 for normal, 2 for warning and 4 for critical. Growth rates require
consecutive measurements: declining swap usage is zero growth, but must not hide
positive swapouts. Missing rates must fail rather than become zero.

The board refuses missing, unknown, non-finite or stale fields. The command has a
two-second deadline and 16 KiB output limit. Errors and timeouts keep worker starts queued;
there is no fallback to a remembered healthy reading. A slow probe may need to
collect its native readings concurrently, but it must not use a stale snapshot.

Admission requires:

- Available memory at least the larger of 6 GiB and 20% of physical memory, plus
  2 GiB for this start and 2 GiB for every other pending start.
- The larger load average, plus 0.8 cores for each pending start and this start,
  no higher than 80% of the core count.
- No critical pressure, and no warning pressure at or below 30% free memory.
  Warning level alone does not refuse admission above 30% free.
- Swap usage growth and swapouts no higher than 1 MiB/s, independently.

The probe and resource reservation are protected by one cross-process board
lock. The reservation persists after that lock is released, so waiting for an
agent to draw its first screen does not block other admissions. Queue claims reserve memory before a detached runner can start. Each
actual launch, including shell-readiness retries, reads the probe again. A
resource collapse during worktree setup therefore leaves the task queued, with
its prepared worktree and exact launch prompt retained. Wakes, inbox delivery
and retries of an unsubmitted prompt use the same gate.

Reservations persist in the board data folder's `admission.json`. They do not
expire just because a start was slow or its runner exited. A worker reservation
is released after the board observes it work, before another fresh probe is
read. A failed or canceled claim can be released if the board never attempted its
launch. A possible launch is different: missing turn data is not proof it
stopped. Such a reservation needs observed worker progress, a fresh confirmed
idle/done observation after its launch attempt, or an explicitly finished task.
A later claim cannot downgrade a possible launch to an unlaunched claim. Each
actual launch attempt starts a new observation epoch; an earlier idle reading
cannot release that new attempt. Missing task records, corrupt reservation
files and uncertain state retain the budget and refuse further starts. Reviewer
reservations are retained until a confirmed stopped or finished task. This is
conservative; do not erase the ledger to recover capacity while starts may be
unconfirmed.

`resourceHold` records a queued task's reason and its row says it is waiting for
host resources. The existing slot count and FIFO order still apply. This change
does not add priority scheduling, a second host governor, or process-curtailment
logic. Running-work intervention belongs to the existing host backstop.

## Verify in a scratch board

Run `npm run check board-queue`. The existing queue tests exercise competing
claims on the same memory budget, resource deterioration before launch, held
wake text, stale or failed probes, persistent reservations and corrupt ledger
handling. A native-probe test checks failure, invalid JSON, excessive output and
a timeout without launching an agent.

Before enabling on a live board, deploy reviewed source, configure a reviewed
read-only probe, and show the low-memory and recovered-resource transitions with
actual agent start records. An isolated setter or a simulation does not prove a
live gate or release a waiting agent's slot.
