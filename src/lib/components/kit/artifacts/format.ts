/**
 * Formatting shared by the artifact cards.
 *
 * The card renderers never format units themselves: an artifact that reports a
 * duration reports milliseconds and an artifact that reports a size reports a
 * label, so the demo data stays readable. Only the conversions every card
 * needs live here.
 */

/** Milliseconds as the kit shows durations: whole ms under a second, one decimal above, then minutes. */
export function formatDuration(ms: number | undefined): string {
	if (ms === undefined) return '—';
	if (ms < 1000) return `${Math.round(ms)} ms`;
	if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s`;
	const minutes = Math.floor(ms / 60_000);
	return `${minutes} m ${Math.round((ms % 60_000) / 1000)} s`;
}

/**
 * Chart axis values stay short: 1,200 rather than 1,200.0, and small fractions
 * keep the one decimal that tells two bars apart.
 */
export function formatAxisValue(value: number): string {
	if (Math.abs(value) >= 1000) return value.toLocaleString(undefined, { maximumFractionDigits: 0 });
	if (Math.abs(value) < 10 && value % 1 !== 0) return value.toFixed(1);
	return String(value);
}