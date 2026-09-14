<script lang="ts">
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

    interface Props {
        status: MetricsStatus
        message: string | null
        snapshot: MetricsSnapshot | null
        history: MetricsSnapshot[]
    }

    let { status, message, snapshot, history }: Props = $props()

    function clamp(value: number, min = 0, max = 100): number {
        return Math.min(max, Math.max(min, value))
    }

    function memoryPercent(value: MetricsSnapshot): number {
        return value.memory_total_bytes > 0
            ? (value.memory_used_bytes / value.memory_total_bytes) * 100
            : 0
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
                const y = 20 - clamp((value / max) * 20, 0, 20)
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

    function statusLabel(): string {
        if (status === 'available') return 'Live · 1s'
        if (status === 'starting') return 'Starting metrics…'
        if (status === 'unavailable') return message ?? 'Metrics unavailable'
        return 'Metrics disabled'
    }
</script>

<div class="metrics-shell px-3 py-2">
    <div class="metrics-heading d-flex align-items-center gap-2 mb-1">
        <span
            class:available={status === 'available'}
            class="status-dot"
        ></span>
        <span class="status-label">{statusLabel()}</span>
        {#if snapshot?.network_interface}
            <span class="ms-auto interface-label"
                >{snapshot.network_interface}</span
            >
        {/if}
    </div>
    {#if snapshot}
        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric-title">CPU</div>
                <div class="metric-value">
                    {snapshot.cpu_percent === null ? '—' : `${snapshot.cpu_percent.toFixed(1)}%`}
                    <span class="metric-sub">{snapshot.cpu_count} CPU</span>
                </div>
                <svg
                    class="sparkline"
                    viewBox="0 0 100 20"
                    preserveAspectRatio="none"
                    aria-hidden="true"
                >
                    <polyline points={cpuPoints}></polyline>
                </svg>
            </div>

            <div class="metric-card">
                <div class="metric-title">MEM</div>
                <div class="metric-value">
                    {memoryPercent(snapshot).toFixed(1)}%
                    <span class="metric-sub">
                        {formatBytes(snapshot.memory_used_bytes)}
                        / {formatBytes(snapshot.memory_total_bytes)}
                    </span>
                </div>
                <svg
                    class="sparkline"
                    viewBox="0 0 100 20"
                    preserveAspectRatio="none"
                    aria-hidden="true"
                >
                    <polyline points={memoryPoints}></polyline>
                </svg>
            </div>

            <div class="metric-card network-card">
                <div class="metric-title">NET</div>
                <div class="metric-value network-value">
                    <span>↓ {formatRate(snapshot.rx_bytes_per_sec)}</span>
                    <span>↑ {formatRate(snapshot.tx_bytes_per_sec)}</span>
                </div>
                <svg
                    class="sparkline"
                    viewBox="0 0 100 20"
                    preserveAspectRatio="none"
                    aria-hidden="true"
                >
                    <polyline class="rx-line" points={rxPoints}></polyline>
                    <polyline class="tx-line" points={txPoints}></polyline>
                </svg>
            </div>
            <div class="metric-card compact-card">
                <div class="metric-title">LOAD</div>
                <div class="metric-value">{snapshot.load1.toFixed(2)}</div>
            </div>

            <div class="metric-card compact-card">
                <div class="metric-title">DISK /</div>
                <div class="metric-value">
                    {snapshot.disk_percent.toFixed(1)}%
                </div>
            </div>

            <div class="metric-card compact-card">
                <div class="metric-title">UP</div>
                <div class="metric-value">
                    {formatUptime(snapshot.uptime_seconds)}
                </div>
            </div>
        </div>
    {:else}
        <div class="metrics-empty">Waiting for the first metrics sample…</div>
    {/if}
</div>

<style lang="scss">
    .metrics-shell {
        flex-shrink: 0;
        margin: 0 10px;
        border-radius: 10px;
        background: rgba(0, 0, 0, 0.38);
        color: rgba(255, 255, 255, 0.9);
        font-variant-numeric: tabular-nums;
    }

    .metrics-heading {
        min-height: 18px;
    }

    .status-dot {
        width: 7px;
        height: 7px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.35);
    }

    .status-dot.available {
        background: #7ee787;
        box-shadow: 0 0 8px rgba(126, 231, 135, 0.45);
    }

    .status-label,
    .interface-label,
    .metrics-empty,
    .metric-sub {
        color: rgba(255, 255, 255, 0.58);
        font-size: 0.72rem;
    }

    .interface-label {
        font-family: monospace;
    }

    .metrics-grid {
        display: grid;
        grid-template-columns: minmax(130px, 1.1fr) minmax(170px, 1.3fr) minmax(220px, 1.7fr) repeat(3, minmax(74px, 0.55fr));
        gap: 8px;
    }

    .metric-card {
        min-width: 0;
        height: 48px;
        padding: 5px 8px;
        border: 1px solid rgba(255, 255, 255, 0.07);
        border-radius: 7px;
        background: rgba(255, 255, 255, 0.035);
        position: relative;
        overflow: hidden;
    }

    .metric-title {
        position: relative;
        z-index: 2;
        color: rgba(255, 255, 255, 0.48);
        font-size: 0.65rem;
        font-weight: 700;
        letter-spacing: 0.06em;
    }

    .metric-value {
        position: relative;
        z-index: 2;
        margin-top: 1px;
        font-size: 0.88rem;
        font-weight: 600;
        white-space: nowrap;
    }

    .metric-sub {
        margin-left: 0.35rem;
        font-weight: 400;
    }

    .network-value {
        display: flex;
        gap: 0.65rem;
        font-size: 0.8rem;
    }

    .sparkline {
        position: absolute;
        inset: auto 0 0 0;
        width: 100%;
        height: 23px;
        opacity: 0.33;
    }

    .sparkline polyline {
        fill: none;
        stroke: currentColor;
        stroke-width: 1.4;
        vector-effect: non-scaling-stroke;
    }

    .sparkline .tx-line {
        stroke-dasharray: 3 2;
        opacity: 0.6;
    }

    .compact-card .metric-value {
        margin-top: 4px;
    }

    @media (max-width: 1100px) {
        .metrics-grid {
            grid-template-columns: repeat(3, minmax(0, 1fr));
        }

        .compact-card {
            height: 40px;
        }
    }

</style>
