<script lang="ts">
	import { onMount } from 'svelte';
	import PixelIcon from './PixelIcon.svelte';

	export let title: string = 'flipctl';

	let time = '';
	let battery = 78;

	onMount(() => {
		const tick = () => {
			const now = new Date();
			const h = String(now.getHours()).padStart(2, '0');
			const m = String(now.getMinutes()).padStart(2, '0');
			time = `${h}:${m}`;
		};
		tick();
		const id = setInterval(tick, 1000);
		return () => clearInterval(id);
	});

	$: batteryFill = Math.round((battery / 100) * 7);
</script>

<header class="status-bar">
	<div class="status-left">
		<span class="dot"></span>
		<span class="title">{title}</span>
	</div>
	<div class="status-right">
		<PixelIcon name="wifi" size={16} color="#ffffff" />
		<span class="time">{time}</span>
		<div class="battery" title="{battery}%">
			<div class="battery-body">
				<div class="battery-fill" style="width: {batteryFill * 2}px;"></div>
			</div>
			<div class="battery-cap"></div>
		</div>
	</div>
</header>

<style>
	.status-bar {
		height: var(--status-h);
		background: var(--status-bg);
		color: var(--status-fg);
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 8px;
		border-bottom: 1px solid #333;
		flex-shrink: 0;
		position: relative;
		z-index: 10;
	}

	.status-left {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.dot {
		width: 6px;
		height: 6px;
		background: var(--accent);
		display: block;
		flex-shrink: 0;
	}

	.title {
		font-size: 16px;
		letter-spacing: 1px;
		color: #ffffff;
		text-transform: uppercase;
	}

	.status-right {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.time {
		font-size: 16px;
		color: #ffffff;
		letter-spacing: 1px;
		min-width: 28px;
	}

	.battery {
		display: flex;
		align-items: center;
		gap: 1px;
	}

	.battery-body {
		width: 16px;
		height: 8px;
		border: 1px solid #ffffff;
		padding: 1px;
		display: flex;
		align-items: center;
	}

	.battery-fill {
		height: 100%;
		background: #ffffff;
		max-width: 12px;
	}

	.battery-cap {
		width: 2px;
		height: 4px;
		background: #ffffff;
	}

	/* Compact mode: 256×144 device LCD (html.compact is set from JS) */
	:global(html.compact) .status-bar {
		padding: 0 4px;
	}

	:global(html.compact) .title,
	:global(html.compact) .time {
		font-size: 8px;
		letter-spacing: 0;
	}

	:global(html.compact) .time {
		min-width: 0;
	}

	:global(html.compact) .dot {
		width: 4px;
		height: 4px;
	}

	:global(html.compact) .status-left,
	:global(html.compact) .status-right {
		gap: 4px;
	}

	/* PixelIcon svg: CSS width/height wins over the size attribute */
	:global(html.compact) .status-right :global(svg) {
		width: 8px;
		height: 8px;
	}

	:global(html.compact) .battery-body {
		width: 10px;
		height: 6px;
	}

	:global(html.compact) .battery-cap {
		height: 3px;
	}
</style>
