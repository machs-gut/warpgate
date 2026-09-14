<script lang="ts">
    import { onDestroy, onMount } from 'svelte'

    interface MetricsSnapshot {
        cpu_percent: number | null
        cpu_count: number
        memory_used_bytes: number
        memory_total_bytes: number
        load1: number
        rx_bytes_per_sec: number | null
        tx_bytes_per_sec: number | null
        network_interface: string | null
        disk_percent: number
        uptime_seconds: number
    }

    type MetricsStatus = 'starting' | 'available' | 'unavailable' | 'disabled'
    type MetricSeverity = 'normal' | 'warning' | 'critical'

    interface Props {
        status: MetricsStatus
        message: string | null
        snapshot: MetricsSnapshot | null
        history: MetricsSnapshot[]
        lastSampleAt: number | null
        layout?: 'compact' | 'rail'
    }

    let {
        status,
        message,
        snapshot,
        history,
        lastSampleAt,
        layout = 'compact',
    }: Props = $props()
    let now = $state(Date.now())
    let clock: ReturnType<typeof setInterval> | null = null

    onMount(() => {
        clock = setInterval(() => {
            now = Date.now()
        }, 1000)
    })

    onDestroy(() => {
        if (clock !== null) clearInterval(clock)
    })

    function clamp(value: number, min = 0, max = 100): number {
        return Math.min(max, Math.max(min, value))
    }

    function memoryPercent(value: MetricsSnapshot): number {
        return value.memory_total_bytes > 0
            ? (value.memory_used_bytes / value.memory_total_bytes) * 100
            : 0
    }

    function loadPercent(value: MetricsSnapshot): number {
        return value.cpu_count > 0 ? (value.load1 / value.cpu_count) * 100 : 0
    }

    function severity(
        value: number,
        warning: number,
        critical: number,
    ): MetricSeverity {
        if (value >= critical) return 'critical'
        if (value >= warning) return 'warning'
        return 'normal'
    }

    function formatBytes(value: number): string {
        const units = ['B', 'KB', 'MB', 'GB', 'TB']
        let size = Math.max(0, value)
        let unit = 0
        while (size >= 1024 && unit < units.length - 1) {
            size /= 1024
            unit += 1
        }
        const digits = size >= 100 || unit === 0 ? 0 : size >= 10 ? 1 : 2
        return `${size.toFixed(digits)} ${units[unit]}`
    }

    function formatRate(value: number | null): string {
        return value === null ? '—' : `${formatBytes(value)}/s`
    }

    function formatUptime(seconds: number): string {
        const totalMinutes = Math.floor(Math.max(0, seconds) / 60)
        const days = Math.floor(totalMinutes / 1440)
        const hours = Math.floor((totalMinutes % 1440) / 60)
        const minutes = totalMinutes % 60
        if (days > 0) return `${days}d ${hours}h`
        if (hours > 0) return `${hours}h ${minutes}m`
        return `${minutes}m`
    }

    function sparkline(values: Array<number | null>, ceiling?: number): string {
        if (values.length < 2) return ''
        const finite = values.map(value => value ?? 0)
        const max = Math.max(ceiling ?? 0, ...finite, 1)
        return finite
            .map((value, index) => {
                const x = (index / (finite.length - 1)) * 100
                const y = 16 - clamp((value / max) * 16, 0, 16)
                return `${x.toFixed(2)},${y.toFixed(2)}`
            })
            .join(' ')
    }

    let cpuPoints = $derived(
        sparkline(
            history.map(value => value.cpu_percent),
            100,
        ),
    )
    let memoryPoints = $derived(
        sparkline(
            history.map(value => memoryPercent(value)),
            100,
        ),
    )
    let networkCeiling = $derived.by(() =>
        Math.max(
            1,
            ...history.flatMap(value => [
                value.rx_bytes_per_sec ?? 0,
                value.tx_bytes_per_sec ?? 0,
            ]),
        ),
    )
    let rxPoints = $derived(
        sparkline(
            history.map(value => value.rx_bytes_per_sec),
            networkCeiling,
        ),
    )
    let txPoints = $derived(
        sparkline(
            history.map(value => value.tx_bytes_per_sec),
            networkCeiling,
        ),
    )

    let sampleAgeMs = $derived(
        lastSampleAt === null ? null : Math.max(0, now - lastSampleAt),
    )
    let stale = $derived(
        status === 'available' && sampleAgeMs !== null && sampleAgeMs > 3500,
    )
    let cpuSeverity = $derived(
        snapshot?.cpu_percent === null || !snapshot
            ? 'normal'
            : severity(snapshot.cpu_percent, 70, 90),
    )
    let memorySeverity = $derived(
        snapshot ? severity(memoryPercent(snapshot), 80, 90) : 'normal',
    )
    let loadSeverity = $derived(
        snapshot ? severity(loadPercent(snapshot), 75, 100) : 'normal',
    )
    let diskSeverity = $derived(
        snapshot ? severity(snapshot.disk_percent, 80, 90) : 'normal',
    )

    function statusLabel(): string {
        if (stale && sampleAgeMs !== null)
            return `STALE · ${Math.floor(sampleAgeMs / 1000)}s`
        if (status === 'available') return 'LIVE'
        if (status === 'starting') return 'STARTING'
        if (status === 'unavailable') return message ?? 'UNAVAILABLE'
        return 'DISABLED'
    }
</script>

<div class:stale class:rail={layout === 'rail'} class="metrics-shell">
    <div
        class="status-block"
        title={message ?? 'Agentless metrics over the current SSH session'}
    >
        <span
            class:available={status === 'available' && !stale}
            class:stale
            class="status-dot"
        ></span>
        <span class="status-label">{statusLabel()}</span>
        {#if snapshot?.network_interface}
            <span class="interface-label">{snapshot.network_interface}</span>
        {/if}
    </div>

    {#if snapshot}
        <div class="metrics-grid">
            <div
                class:warning={cpuSeverity === 'warning'}
                class:critical={cpuSeverity === 'critical'}
                class="metric-card"
                title="CPU utilization · warning ≥70%, critical ≥90%"
            >
                <span class="metric-title">CPU</span>
                <span class="metric-value"
                    >{snapshot.cpu_percent === null ? '—' : `${snapshot.cpu_percent.toFixed(1)}%`}</span
                >
                <span class="metric-sub">{snapshot.cpu_count}C</span>
                <svg
                    class="sparkline"
                    viewBox="0 0 100 16"
                    preserveAspectRatio="none"
                    aria-hidden="true"
                >
                    <polyline points={cpuPoints}></polyline>
                </svg>
            </div>

            <div
                class:warning={memorySeverity === 'warning'}
                class:critical={memorySeverity === 'critical'}
                class="metric-card"
                title={`Memory · ${formatBytes(snapshot.memory_used_bytes)} / ${formatBytes(snapshot.memory_total_bytes)} · warning ≥80%, critical ≥90%`}
            >
                <span class="metric-title">MEM</span>
                <span class="metric-value"
                    >{memoryPercent(snapshot).toFixed(1)}%</span
                >
                <span class="metric-sub"
                    >{formatBytes(snapshot.memory_used_bytes)}
                    / {formatBytes(snapshot.memory_total_bytes)}</span
                >
                <svg
                    class="sparkline"
                    viewBox="0 0 100 16"
                    preserveAspectRatio="none"
                    aria-hidden="true"
                >
                    <polyline points={memoryPoints}></polyline>
                </svg>
            </div>

            <div
                class="metric-card network-card"
                title={snapshot.network_interface ? `Network interface: ${snapshot.network_interface}` : 'Network throughput'}
            >
                <span class="metric-title">NET</span>
                <span class="metric-value network-value"
                    ><span>↓ {formatRate(snapshot.rx_bytes_per_sec)}</span
                    ><span
                        >↑ {formatRate(snapshot.tx_bytes_per_sec)}</span
                    ></span
                >
                <svg
                    class="sparkline"
                    viewBox="0 0 100 16"
                    preserveAspectRatio="none"
                    aria-hidden="true"
                >
                    <polyline class="rx-line" points={rxPoints}></polyline>
                    <polyline class="tx-line" points={txPoints}></polyline>
                </svg>
            </div>

            <div
                class:warning={loadSeverity === 'warning'}
                class:critical={loadSeverity === 'critical'}
                class="metric-card compact-card"
                title="1-minute load normalized by logical CPU count · warning ≥75%, critical ≥100%"
            >
                <span class="metric-title">LOAD</span>
                <span class="metric-value"
                    >{snapshot.load1.toFixed(2)}
                    / {snapshot.cpu_count}C</span
                >
                <span class="metric-sub"
                    >· {loadPercent(snapshot).toFixed(0)}%</span
                >
            </div>

            <div
                class:warning={diskSeverity === 'warning'}
                class:critical={diskSeverity === 'critical'}
                class="metric-card compact-card"
                title="Root filesystem usage · warning ≥80%, critical ≥90%"
            >
                <span class="metric-title">DISK /</span>
                <span class="metric-value"
                    >{snapshot.disk_percent.toFixed(1)}%</span
                >
            </div>

            <div class="metric-card compact-card" title="Remote host uptime">
                <span class="metric-title">UP</span>
                <span class="metric-value"
                    >{formatUptime(snapshot.uptime_seconds)}</span
                >
            </div>
        </div>
    {:else}
        <div class="metrics-empty">Waiting for metrics…</div>
    {/if}
</div>

<style lang="scss">
    .metrics-shell {
        flex-shrink: 0;
        display: flex;
        align-items: stretch;
        gap: 6px;
        min-height: 36px;
        margin: 6px 10px 0;
        padding: 4px 6px;
        border-radius: 9px;
        background: rgba(0, 0, 0, 0.38);
        color: rgba(255, 255, 255, 0.9);
        font-variant-numeric: tabular-nums;
    }

    .metrics-shell.stale .metrics-grid {
        opacity: 0.52;
        filter: saturate(0.35);
    }

    .status-block {
        min-width: 82px;
        display: flex;
        align-items: center;
        gap: 5px;
        padding: 0 5px;
    }

    .status-dot {
        width: 7px;
        height: 7px;
        flex: 0 0 auto;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.35);
    }

    .status-dot.available {
        background: #7ee787;
        box-shadow: 0 0 7px rgba(126, 231, 135, 0.45);
    }

    .status-dot.stale {
        background: #d29922;
        box-shadow: 0 0 7px rgba(210, 153, 34, 0.38);
    }

    .status-label,
    .interface-label,
    .metrics-empty,
    .metric-sub {
        color: rgba(255, 255, 255, 0.56);
        font-size: 0.66rem;
    }

    .status-label {
        font-weight: 700;
        letter-spacing: 0.04em;
    }

    .interface-label {
        display: none;
        font-family: monospace;
    }

    .metrics-empty {
        display: flex;
        align-items: center;
    }

    .metrics-grid {
        min-width: 0;
        flex: 1 1 auto;
        display: grid;
        grid-template-columns: minmax(118px, 1fr) minmax(185px, 1.35fr) minmax(220px, 1.6fr) minmax(145px, 1fr) minmax(82px, 0.6fr) minmax(82px, 0.6fr);
        gap: 5px;
    }

    .metric-card {
        min-width: 0;
        height: 28px;
        display: flex;
        align-items: center;
        gap: 5px;
        padding: 3px 7px;
        border: 1px solid rgba(255, 255, 255, 0.07);
        border-radius: 6px;
        background: rgba(255, 255, 255, 0.035);
        position: relative;
        overflow: hidden;
        transition: border-color 120ms ease, background 120ms ease;
    }

    .metric-card.warning {
        border-color: rgba(210, 153, 34, 0.52);
        background: rgba(210, 153, 34, 0.11);
    }

    .metric-card.critical {
        border-color: rgba(248, 81, 73, 0.62);
        background: rgba(248, 81, 73, 0.14);
    }

    .metric-card.warning .metric-value,
    .metric-card.warning .metric-title {
        color: #e3b341;
    }

    .metric-card.critical .metric-value,
    .metric-card.critical .metric-title {
        color: #ff7b72;
    }

    .metric-title,
    .metric-value,
    .metric-sub {
        position: relative;
        z-index: 2;
        white-space: nowrap;
    }

    .metric-title {
        color: rgba(255, 255, 255, 0.46);
        font-size: 0.62rem;
        font-weight: 700;
        letter-spacing: 0.05em;
    }

    .metric-value {
        font-size: 0.78rem;
        font-weight: 650;
    }

    .metric-sub {
        overflow: hidden;
        text-overflow: ellipsis;
        font-weight: 400;
    }

    .network-value {
        display: flex;
        gap: 0.55rem;
        font-size: 0.74rem;
    }

    .sparkline {
        position: absolute;
        inset: auto 0 0 0;
        width: 100%;
        height: 16px;
        opacity: 0.28;
        pointer-events: none;
    }

    .sparkline polyline {
        fill: none;
        stroke: currentColor;
        stroke-width: 1.2;
        vector-effect: non-scaling-stroke;
    }

    .sparkline .tx-line {
        stroke-dasharray: 3 2;
        opacity: 0.6;
    }

    @media (min-width: 1450px) {
        .interface-label {
            display: inline;
        }
    }

    @media (max-width: 1100px) {
        .metrics-shell {
            align-items: flex-start;
        }

        .metrics-grid {
            grid-template-columns: repeat(3, minmax(0, 1fr));
        }

        .status-block {
            min-height: 28px;
        }
    }
    .metrics-shell.rail {
        width: 100%;
        height: 100%;
        min-height: 0;
        margin: 0;
        padding: 8px;
        flex-direction: column;
        align-items: stretch;
        gap: 8px;
        border-radius: 0;
        background: transparent;
        overflow-y: auto;
    }

    .rail .status-block {
        min-width: 0;
        min-height: 34px;
        padding: 3px 4px 8px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    }

    .rail .interface-label {
        display: inline;
        margin-left: auto;
    }

    .rail .metrics-grid {
        display: flex;
        flex: 0 0 auto;
        flex-direction: column;
        gap: 8px;
    }

    .rail .metric-card {
        width: 100%;
        height: 72px;
        min-height: 72px;
        display: grid;
        grid-template-columns: auto 1fr;
        grid-template-rows: auto auto 1fr;
        column-gap: 8px;
        align-items: baseline;
        padding: 8px 9px;
    }

    .rail .metric-title {
        grid-column: 1;
        grid-row: 1;
        font-size: 0.64rem;
    }

    .rail .metric-value {
        grid-column: 2;
        grid-row: 1;
        justify-self: end;
        font-size: 0.86rem;
    }

    .rail .metric-sub {
        grid-column: 1 / -1;
        grid-row: 2;
        margin-top: 2px;
    }

    .rail .network-value {
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: 0;
        line-height: 1.25;
    }

    .rail .sparkline {
        height: 28px;
        bottom: 5px;
        opacity: 0.36;
    }

    .rail .compact-card {
        height: 54px;
        min-height: 54px;
        grid-template-rows: auto 1fr;
    }

    .rail .compact-card .metric-sub {
        grid-column: 1 / -1;
        grid-row: 2;
    }

    .rail .metrics-empty {
        padding: 12px 4px;
    }

</style>
