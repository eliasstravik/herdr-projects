// Command Code's per-launch turn callback, including errors and interruptions that do not fire Stop hooks.
import { spawn } from 'node:child_process';
const command = __SW_COMMAND__;
export default function(cmd) {
    let session = '', error = null;
    cmd.hooks({ onSessionStart: ({ sessionId }) => { session = sessionId || ''; } });
    cmd.on('run_start', event => {
        session = event.sessionId || session;
        error = null;
        cmd.session?.appendCustomEntry({ customType: 'herdr-projects/run-start' });
    });
    cmd.on('run_error', event => { error = event.error; });
    for (const type of ['subagent_start', 'subagent_stop']) {
        cmd.on(type, event => cmd.session?.appendCustomEntry({ customType: 'herdr-projects/helper', data: {
            id: event.toolCallId, running: type === 'subagent_start',
        } }));
    }
    cmd.hooks({ onRunEnd: async ({ state, result }) => {
        if (session && state.sessionId !== session) return;
        const report = {
            session_id: session || state.sessionId,
            finished_at: Date.now(),
            stop: result.stopReason,
            error: error?.message || '', code: error?.code || '', status: error?.status || 0,
        };
        cmd.session?.appendCustomEntry({ customType: 'herdr-projects/run-end', data: {
            ...report, finalText: result.finalText,
        } });
        // The store queues its append; leave the microtask queue before waking the board, which also retries reads.
        await new Promise(resolve => setImmediate(resolve));
        try {
            const child = spawn(command[0], command.slice(1), { stdio: ['pipe', 'ignore', 'ignore'], detached: true });
            child.on('error', () => {});
            child.stdin.on('error', () => {});
            child.stdin.end(JSON.stringify(report));
            child.unref();
        } catch {}
    } });
}
