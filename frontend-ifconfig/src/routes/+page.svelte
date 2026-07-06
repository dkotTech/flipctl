<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	interface NetInterface {
		name: string;
		ip4: string | null;
		ip6: string | null;
		status: 'UP' | 'DOWN';
		mac: string | null;
		rx: string | null;
		tx: string | null;
	}

	const REFRESH_OPTS = [0, 1, 2, 3, 5, 10]; // seconds, 0 = off
	type StatusFilter = 'all' | 'up';
	type Family = 'all' | 'v4' | 'v6';

	let refreshS = 3;
	let statusFilter: StatusFilter = 'all';
	let family: Family = 'all';

	let interfaces: NetInterface[] = [];
	let selected = 0;
	let error: string | null = null;
	let loaded = false;
	let timer: ReturnType<typeof setInterval> | undefined;

	$: activeCount = interfaces.filter((i) => i.status === 'UP').length;
	$: shown = interfaces.filter((i) => statusFilter === 'all' || i.status === 'UP');
	$: if (selected >= shown.length) selected = Math.max(0, shown.length - 1);

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		const units = ['KB', 'MB', 'GB', 'TB'];
		let value = bytes;
		let unit = -1;
		do {
			value /= 1024;
			unit++;
		} while (value >= 1024 && unit < units.length - 1);
		return `${value.toFixed(1)} ${units[unit]}`;
	}

	// Maps one entry of `ip -j -s addr show` output to our view model.
	function toInterface(link: any): NetInterface {
		const addrs: any[] = link.addr_info ?? [];
		const ip4 = addrs.find((a) => a.family === 'inet');
		const ip6 = addrs.find((a) => a.family === 'inet6');
		const flags: string[] = link.flags ?? [];
		const up =
			link.operstate === 'UP' ||
			(link.operstate === 'UNKNOWN' && flags.includes('UP'));
		return {
			name: link.ifname,
			ip4: ip4 ? `${ip4.local}/${ip4.prefixlen}` : null,
			ip6: ip6 ? `${ip6.local}/${ip6.prefixlen}` : null,
			status: up ? 'UP' : 'DOWN',
			mac: link.link_type === 'loopback' ? null : (link.address ?? null),
			rx: link.stats64 ? formatBytes(link.stats64.rx.bytes) : null,
			tx: link.stats64 ? formatBytes(link.stats64.tx.bytes) : null
		};
	}

	async function refresh() {
		try {
			const res = await fetch('/api/exec', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ cmd: 'ip', args: ['-j', '-s', 'addr', 'show'] })
			});
			if (!res.ok) throw new Error(`api: HTTP ${res.status}`);
			const out = await res.json();
			if (out.exit_code !== 0) throw new Error(out.stderr || `exit ${out.exit_code}`);
			interfaces = JSON.parse(out.stdout).map(toInterface);
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loaded = true;
		}
	}

	function setupTimer() {
		clearInterval(timer);
		timer = refreshS > 0 ? setInterval(refresh, refreshS * 1000) : undefined;
	}

	// Restart the poll loop whenever the interval changes (incl. via <select>).
	$: refreshS, setupTimer();

	onMount(() => {
		// html.compact for small screens: servo doesn't match @media (max-height)
		// against the real webview size, so this is driven from JS.
		document.documentElement.classList.toggle('compact', window.innerHeight <= 240);
		refresh();
	});
	onDestroy(() => {
		clearInterval(timer);
	});
</script>

<div class="ifconfig-app">
	<!-- Header bar -->
	<div class="header">
		<div class="header-left">
			<span class="dot"></span>
			<span class="title">ifconfig</span>
		</div>
		<div class="header-right">
			<span class="active-count">{activeCount} UP</span>
		</div>
	</div>

	<!-- Options bar -->
	<div class="opts">
		<div class="opt">
			<span class="o-key">EVERY</span>
			<select class="o-select" bind:value={refreshS}>
				{#each REFRESH_OPTS as v}<option value={v}>{v === 0 ? 'OFF' : `${v}s`}</option>{/each}
			</select>
		</div>
		<div class="opt">
			<span class="o-key">SHOW</span>
			<select class="o-select" bind:value={statusFilter}>
				<option value="all">ALL</option>
				<option value="up">UP</option>
			</select>
		</div>
		<div class="opt">
			<span class="o-key">IP</span>
			<select class="o-select" bind:value={family}>
				<option value="all">ALL</option>
				<option value="v4">v4</option>
				<option value="v6">v6</option>
			</select>
		</div>
	</div>

	<!-- Interface list -->
	<div class="iface-list">
		{#if !loaded}
			<div class="notice">LOADING...</div>
		{:else if error && !interfaces.length}
			<div class="notice error">{error}</div>
		{:else if !shown.length}
			<div class="notice">NO INTERFACES</div>
		{/if}

		{#each shown as iface, i}
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<div
				class="iface-block"
				class:selected={selected === i}
				on:click={() => (selected = i)}
				on:mouseenter={() => (selected = i)}
				role="button"
				tabindex="-1"
			>
				<!-- Top row: name + status -->
				<div class="iface-top">
					<span class="iface-name">{iface.name}</span>
					<span class="iface-status" class:up={iface.status === 'UP'}>
						{iface.status}
					</span>
				</div>

				<!-- Details: fixed key column on the 8px grid -->
				<div class="iface-details">
					{#if iface.ip4 && family !== 'v6'}
						<span class="detail-key">IP4</span>
						<span class="detail-val">{iface.ip4}</span>
					{/if}

					{#if iface.ip6 && family !== 'v4'}
						<span class="detail-key">IP6</span>
						<span class="detail-val">{iface.ip6}</span>
					{/if}

					{#if iface.mac}
						<span class="detail-key">MAC</span>
						<span class="detail-val">{iface.mac}</span>
					{/if}

					{#if iface.rx !== null}
						<span class="detail-key">RX</span>
						<span class="detail-val">{iface.rx}<span class="sep"> / </span>{iface.tx} TX</span>
					{/if}
				</div>
			</div>
		{/each}
	</div>

	<!-- Footer hint -->
	<div class="footer">
		<span class="hint">{shown.length} SHOWN</span>
		<span class="hint-right" class:err={error !== null}>
			{error ? 'OFFLINE' : refreshS === 0 ? 'PAUSED' : 'LIVE'}
		</span>
	</div>
</div>

<style>
	/*
	 * Pixel-perfect rules:
	 * - Press Start 2P is drawn on an 8x8 grid → font size 8px only
	 * - no letter-spacing, heights/paddings on a 4px grid
	 * - flat colors only, selection = full inversion, no transitions
	 */

	.ifconfig-app {
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

	.active-count {
		color: var(--accent);
	}

	/* Options bar */
	.opts {
		display: flex;
		gap: 8px;
		padding: 8px;
		border-bottom: 1px solid var(--lcd-dark);
		flex-shrink: 0;
	}

	.opt {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.o-key {
		color: #83856a;
	}

	.o-select {
		font: inherit;
		font-size: 8px;
		color: var(--pixel);
		background: var(--lcd);
		border: 1px solid var(--lcd-dark);
		outline: none;
		height: 16px;
		padding: 0 2px;
		border-radius: 0;
		cursor: pointer;
		-webkit-appearance: none;
		-moz-appearance: none;
		appearance: none;
	}

	.o-select:focus {
		border-color: var(--pixel);
	}

	/* Interface list */
	.iface-list {
		flex: 1;
		overflow-y: auto;
		overflow-x: hidden;
	}

	.notice {
		padding: 16px 8px;
		color: #62644a;
	}

	.notice.error {
		color: #883322;
	}

	.iface-block {
		padding: 8px;
		border-bottom: 1px solid var(--lcd-dark);
		cursor: pointer;
	}

	.iface-block.selected {
		background: var(--selected-bg);
		color: var(--selected-fg);
		border-bottom-color: var(--selected-bg);
	}

	/* Top row: name + badge */
	.iface-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin-bottom: 8px;
	}

	.iface-name {
		font-size: 8px;
		line-height: 8px;
		text-transform: lowercase;
		overflow: hidden;
		white-space: nowrap;
	}

	.iface-status {
		flex-shrink: 0;
		padding: 3px 4px 1px;
		border: 1px solid currentColor;
		color: #83856a;
	}

	.iface-status.up {
		color: var(--pixel);
	}

	.iface-block.selected .iface-status {
		color: var(--selected-fg);
	}

	.iface-block.selected .iface-status.up {
		color: var(--selected-bg);
		background: var(--selected-fg);
		border-color: var(--selected-fg);
	}

	/* Details: 32px key column + value, rows on a 16px rhythm */
	.iface-details {
		display: grid;
		grid-template-columns: 32px 1fr;
		column-gap: 8px;
		align-items: baseline;
	}

	.detail-key {
		line-height: 16px;
		color: #83856a;
	}

	.detail-val {
		line-height: 16px;
		word-break: break-all;
	}

	.sep {
		color: #83856a;
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
	:global(html.compact) .header,
	:global(html.compact) .footer {
		height: 16px;
		padding: 0 4px;
	}

	:global(html.compact) .opts {
		gap: 4px;
		padding: 4px;
	}

	:global(html.compact) .o-select {
		height: 12px;
	}

	:global(html.compact) .iface-block {
		padding: 4px;
	}

	:global(html.compact) .iface-top {
		margin-bottom: 4px;
	}

	:global(html.compact) .iface-details {
		grid-template-columns: 24px 1fr;
		column-gap: 4px;
	}

	:global(html.compact) .detail-key,
	:global(html.compact) .detail-val {
		line-height: 12px;
	}

	:global(html.compact) .notice {
		padding: 8px 4px;
	}
</style>
