<script lang="ts">
	import { onMount } from 'svelte';

	interface Reply {
		seq: number;
		time: number; // ms
	}

	interface Summary {
		tx: number;
		rx: number;
		loss: number; // percent
		min: number;
		avg: number;
		max: number;
	}

	const PRESETS = ['8.8.8.8', '1.1.1.1', '192.168.0.1', 'google.com'];

	// Adjustable ping parameters, each mapped to a real flag.
	const COUNT_OPTS = [1, 2, 3, 4, 5, 10];
	const SIZE_OPTS = [16, 32, 56, 64, 128, 256, 512, 1024];
	const INTERVAL_OPTS = [0.2, 0.5, 1, 2];
	const TIMEOUT_OPTS = [1, 2, 3, 5];

	let host = PRESETS[0];
	let count = 4;
	let size = 56;
	let interval = 1;
	let timeout = 2;

	let running = false;
	let replies: Reply[] = [];
	let summary: Summary | null = null;
	let error: string | null = null;
	let ran = false;

	$: maxTime = replies.reduce((m, r) => Math.max(m, r.time), 1);

	// Parse iputils `ping -c N` output into replies + a summary line.
	function parse(stdout: string): { replies: Reply[]; summary: Summary | null } {
		const replies: Reply[] = [];
		for (const line of stdout.split('\n')) {
			const m = line.match(/icmp_seq=(\d+).*?time=([\d.]+)/);
			if (m) replies.push({ seq: Number(m[1]), time: Number(m[2]) });
		}

		let summary: Summary | null = null;
		const stat = stdout.match(/(\d+) packets transmitted, (\d+) received.*?([\d.]+)% packet loss/s);
		const rtt = stdout.match(/= ([\d.]+)\/([\d.]+)\/([\d.]+)/);
		if (stat) {
			summary = {
				tx: Number(stat[1]),
				rx: Number(stat[2]),
				loss: Number(stat[3]),
				min: rtt ? Number(rtt[1]) : 0,
				avg: rtt ? Number(rtt[2]) : 0,
				max: rtt ? Number(rtt[3]) : 0
			};
		}
		return { replies, summary };
	}

	async function run() {
		const target = host.trim();
		if (!target || running) return;
		running = true;
		ran = true;
		replies = [];
		summary = null;
		error = null;
		try {
			const args = [
				'-c', String(count),
				'-s', String(size),
				'-i', String(interval),
				'-W', String(timeout),
				target
			];
			const res = await fetch('/api/exec', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ cmd: 'ping', args })
			});
			if (!res.ok) throw new Error(`api: HTTP ${res.status}`);
			const out = await res.json();
			const parsed = parse(out.stdout);
			replies = parsed.replies;
			summary = parsed.summary;
			// ping exits non-zero on 100% loss / unknown host: surface stderr,
			// but keep whatever partial replies/summary we managed to parse.
			if (out.exit_code !== 0 && !replies.length) {
				error = (out.stderr || out.stdout).trim().split('\n')[0] || `exit ${out.exit_code}`;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			running = false;
		}
	}

	function onHostKey(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			run();
		}
	}

	$: statusLabel = running ? 'PING...' : error ? 'FAIL' : summary ? `${summary.loss}% LOSS` : 'READY';

	onMount(() => {
		// html.compact for small screens: servo doesn't match @media (max-height)
		// against the real webview size, so this is driven from JS.
		document.documentElement.classList.toggle('compact', window.innerHeight <= 240);
	});
</script>

<div class="ping-app">
	<!-- Header -->
	<div class="header">
		<div class="header-left">
			<span class="dot"></span>
			<span class="title">ping</span>
		</div>
		<span class="status" class:err={error}>{statusLabel}</span>
	</div>

	<!-- Parameter form -->
	<div class="menu">
		<div class="field">
			<span class="f-key">HOST</span>
			<input
				class="f-input"
				bind:value={host}
				list="host-presets"
				spellcheck="false"
				on:keydown={onHostKey}
			/>
			<datalist id="host-presets">
				{#each PRESETS as p}<option value={p}></option>{/each}
			</datalist>
		</div>

		<div class="field">
			<span class="f-key">COUNT</span>
			<select class="f-select" bind:value={count}>
				{#each COUNT_OPTS as v}<option value={v}>{v}</option>{/each}
			</select>
		</div>

		<div class="field">
			<span class="f-key">SIZE</span>
			<select class="f-select" bind:value={size}>
				{#each SIZE_OPTS as v}<option value={v}>{v} B</option>{/each}
			</select>
		</div>

		<div class="field">
			<span class="f-key">INTVL</span>
			<select class="f-select" bind:value={interval}>
				{#each INTERVAL_OPTS as v}<option value={v}>{v} s</option>{/each}
			</select>
		</div>

		<div class="field">
			<span class="f-key">TMOUT</span>
			<select class="f-select" bind:value={timeout}>
				{#each TIMEOUT_OPTS as v}<option value={v}>{v} s</option>{/each}
			</select>
		</div>

		<button class="run-btn" on:click={run} disabled={running}>
			{running ? 'PINGING...' : 'PING'}
		</button>
	</div>

	<!-- Results -->
	<div class="results">
		{#if !ran}
			<div class="notice">SET PARAMS &#183; PRESS PING</div>
		{:else if running && !replies.length}
			<div class="notice">PINGING {host}...</div>
		{:else if error && !replies.length}
			<div class="notice error">{error}</div>
		{/if}

		{#each replies as r}
			<div class="reply">
				<span class="seq">#{r.seq}</span>
				<div class="bar-track">
					<div class="bar" style="width: {Math.max(4, (r.time / maxTime) * 100)}%"></div>
				</div>
				<span class="ms">{r.time.toFixed(1)} ms</span>
			</div>
		{/each}

		{#if summary}
			<div class="summary">
				<div class="sum-row">
					<span class="sum-key">SENT</span><span class="sum-val">{summary.tx}</span>
					<span class="sum-key">RECV</span><span class="sum-val">{summary.rx}</span>
					<span class="sum-key">LOSS</span>
					<span class="sum-val" class:bad={summary.loss > 0}>{summary.loss}%</span>
				</div>
				{#if summary.rx > 0}
					<div class="sum-row">
						<span class="sum-key">MIN</span><span class="sum-val">{summary.min}</span>
						<span class="sum-key">AVG</span><span class="sum-val">{summary.avg}</span>
						<span class="sum-key">MAX</span><span class="sum-val">{summary.max}</span>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Footer -->
	<div class="footer">
		<span class="hint">ENTER = PING</span>
		<span class="hint-right" class:err={error !== null}>{running ? 'BUSY' : 'IDLE'}</span>
	</div>
</div>

<style>
	/*
	 * Pixel-perfect: Press Start 2P on an 8x8 grid.
	 * font size 8px only, no letter-spacing, geometry on a 4px grid,
	 * flat colors, no transitions.
	 */

	.ping-app {
		position: fixed;
		inset: 0;
		display: flex;
		flex-direction: column;
		background: var(--lcd);
		overflow: hidden;
		font-size: 8px;
		line-height: 8px;
	}

	/* Header */
	.header {
		height: 24px;
		background: var(--status-bg);
		color: var(--status-fg);
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 8px;
		flex-shrink: 0;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.dot {
		width: 4px;
		height: 4px;
		background: var(--accent);
		display: block;
	}

	.title {
		text-transform: uppercase;
	}

	.status {
		color: var(--accent);
	}

	.status.err {
		color: #dd4433;
	}

	/* Parameter form */
	.menu {
		flex-shrink: 0;
		border-bottom: 1px solid var(--lcd-dark);
	}

	.field {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		height: 24px;
		padding: 0 8px;
		border-bottom: 1px solid var(--lcd-dark);
	}

	.f-key {
		color: #83856a;
		flex-shrink: 0;
	}

	/* Native controls, restyled to the LCD theme */
	.f-input,
	.f-select {
		font: inherit;
		font-size: 8px;
		color: var(--pixel);
		background: var(--lcd);
		border: 1px solid var(--lcd-dark);
		outline: none;
		height: 16px;
		padding: 0 4px;
		border-radius: 0;
		cursor: pointer;
	}

	.f-input {
		flex: 1;
		max-width: 60%;
		text-align: right;
		cursor: text;
	}

	.f-select {
		min-width: 64px;
		text-align: right;
		-webkit-appearance: none;
		-moz-appearance: none;
		appearance: none;
		text-align-last: right;
	}

	.f-input:focus,
	.f-select:focus {
		border-color: var(--pixel);
	}

	.run-btn {
		display: block;
		width: 100%;
		height: 24px;
		font: inherit;
		font-size: 8px;
		text-transform: uppercase;
		color: var(--selected-fg);
		background: var(--selected-bg);
		border: none;
		cursor: pointer;
	}

	.run-btn:hover {
		color: var(--accent);
	}

	.run-btn:disabled {
		color: #83856a;
		cursor: default;
	}

	/* Results */
	.results {
		flex: 1;
		overflow-y: auto;
		overflow-x: hidden;
		padding: 4px 0;
	}

	.notice {
		padding: 12px 8px;
		color: #62644a;
	}

	.notice.error {
		color: #883322;
	}

	.reply {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 8px;
	}

	.seq {
		width: 20px;
		flex-shrink: 0;
		color: #83856a;
	}

	.bar-track {
		flex: 1;
		height: 8px;
		background: var(--lcd-dark);
	}

	.bar {
		height: 8px;
		background: var(--pixel);
	}

	.ms {
		width: 56px;
		flex-shrink: 0;
		text-align: right;
	}

	/* Summary */
	.summary {
		margin: 4px 8px 0;
		padding: 8px;
		border-top: 1px solid var(--lcd-dark);
	}

	.sum-row {
		display: grid;
		grid-template-columns: auto 1fr auto 1fr auto 1fr;
		align-items: baseline;
		column-gap: 6px;
		margin-bottom: 8px;
	}

	.sum-row:last-child {
		margin-bottom: 0;
	}

	.sum-key {
		color: #83856a;
	}

	.sum-val {
		line-height: 8px;
	}

	.sum-val.bad {
		color: #dd4433;
	}

	/* Footer */
	.footer {
		height: 24px;
		background: var(--action-bg);
		color: var(--action-fg);
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 8px;
		flex-shrink: 0;
	}

	.hint-right {
		color: var(--accent);
	}

	.hint-right.err {
		color: #dd4433;
	}

	/* Compact mode: 256×144 device LCD (html.compact is set from JS) */
	:global(html.compact) .header {
		height: 16px;
		padding: 0 4px;
	}

	:global(html.compact) .field {
		height: 16px;
		padding: 0 4px;
		gap: 4px;
	}

	:global(html.compact) .f-input,
	:global(html.compact) .f-select {
		height: 12px;
		padding: 0 2px;
	}

	:global(html.compact) .run-btn {
		height: 16px;
	}

	:global(html.compact) .results {
		padding: 2px 0;
	}

	:global(html.compact) .notice {
		padding: 6px 4px;
	}

	:global(html.compact) .reply {
		padding: 2px 4px;
		gap: 4px;
	}

	:global(html.compact) .summary {
		margin: 2px 4px 0;
		padding: 4px;
	}

	:global(html.compact) .sum-row {
		margin-bottom: 4px;
	}

	:global(html.compact) .footer {
		display: none;
	}
</style>
