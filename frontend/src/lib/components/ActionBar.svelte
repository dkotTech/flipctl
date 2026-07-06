<script lang="ts">
	import { leftAction, rightAction } from '$lib/stores/actions';
</script>

<footer class="action-bar">
	<div class="action-slot left">
		{#if $leftAction}
			<button class="action-btn" on:click>
				<span class="action-key">{$leftAction.key}</span>
				<span class="action-label">{$leftAction.label}</span>
			</button>
		{/if}
	</div>

	<div class="action-slot right">
		{#if $rightAction}
			<button class="action-btn accent" on:click>
				<span class="action-key">{$rightAction.key}</span>
				<span class="action-label">{$rightAction.label}</span>
			</button>
		{/if}
	</div>
</footer>

<style>
	.action-bar {
		height: var(--action-h);
		background: var(--action-bg);
		border-top: 1px solid #2a2a2a;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 10px;
		flex-shrink: 0;
	}

	.action-slot {
		display: flex;
		align-items: center;
	}

	.action-btn {
		background: none;
		border: none;
		display: flex;
		align-items: center;
		gap: 5px;
		cursor: pointer;
		font-family: 'Press Start 2P', monospace;
		padding: 4px 6px;
	}

	.action-key {
		font-size: 13px;
		color: var(--accent);
		display: block;
	}

	.action-label {
		font-size: 9px;
		color: var(--action-fg);
		display: block;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.action-btn.accent .action-key {
		color: var(--accent);
	}

	/* Compact mode: 256×144 device LCD (html.compact is set from JS) */
	:global(html.compact) .action-bar {
		padding: 0 4px;
	}

	:global(html.compact) .action-btn {
		gap: 3px;
		padding: 0 2px;
	}

	:global(html.compact) .action-key {
		font-size: 8px;
	}

	:global(html.compact) .action-label {
		font-size: 8px;
		letter-spacing: 0;
	}
</style>
