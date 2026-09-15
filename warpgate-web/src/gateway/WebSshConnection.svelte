<script lang="ts">
    import {
        Button,
        Modal,
        ModalBody,
        ModalFooter,
    } from '@sveltestrap/sveltestrap'
    import InfoBox from 'common/InfoBox.svelte'
    import { onDestroy, onMount, tick } from 'svelte'
    import { SvelteMap } from 'svelte/reactivity'
    import { api, ResponseError, type WebSshSessionInfo } from './lib/api'
    import {
        ConnectionState,
        ReconnectingWebSocket,
    } from './lib/ReconnectingWebSocket.svelte'
    import SshTerminalTab from './WebSshTab.svelte'
    import type { TerminalTheme } from './WebSshThemes'
    import type {
        MetricsSnapshot,
        MetricsStatus,
        MetricsViewState,
        ReconnectReason,
        TerminalHistorySnapshot,
    } from './WebSshTypes'

    interface Props {
        sessionId: string
        active: boolean
        fontSize: number
        theme: TerminalTheme
        initialHistory?: TerminalHistorySnapshot | null
        onInfo: (sessionId: string, info: WebSshSessionInfo) => void
        onConnectionState: (
            sessionId: string,
            state: ConnectionState,
            attempt: number,
        ) => void
        onMetrics: (sessionId: string, metrics: MetricsViewState) => void
        onTransportReconnected: (sessionId: string, attempts: number) => void
        onTargetConnected: (sessionId: string) => void
        onReconnectNeeded: (sessionId: string, reason: ReconnectReason) => void
        onError: (
            sessionId: string,
            message: string | null,
            notFound: boolean,
        ) => void
    }

    let {
        sessionId,
        active,
        fontSize,
        theme,
        initialHistory = null,
        onInfo,
        onConnectionState,
        onMetrics,
        onTransportReconnected,
        onTargetConnected,
        onReconnectNeeded,
        onError,
    }: Props = $props()

    type ClientMessage =
        | { type: 'open_channel'; cols?: number; rows?: number }
        | { type: 'input'; channel_id: string; data: string }
        | { type: 'resize'; channel_id: string; cols: number; rows: number }
        | { type: 'close_channel'; channel_id: string }
        | { type: 'accept_host_key' }
        | { type: 'reject_host_key' }
        | { type: 'start_metrics' }
        | { type: 'stop_metrics' }

    type ServerMessage =
        | { type: 'connection_state'; state: ConnectionState }
        | { type: 'output'; channel_id: string; data: string }
        | { type: 'channel_opened'; channel_id: string }
        | { type: 'channel_closed'; channel_id: string }
        | { type: 'eof'; channel_id: string }
        | { type: 'exit_status'; channel_id: string; code: number }
        | { type: 'error'; message: string }
        | {
              type: 'host_key_unknown'
              host: string
              port: number
              key_type: string
              key_base64: string
          }
        | {
              type: 'metrics_status'
              state: MetricsStatus
              message: string | null
          }
        | { type: 'metrics_snapshot'; snapshot: MetricsSnapshot }

    interface ChannelState {
        id: string
        label: string
        terminalTitle: string | undefined
        closed: boolean
    }

    let channels = new SvelteMap<string, ChannelState>()
    let channelOrder: string[] = $state([])
    let activeChannelId: string | null = $state(null)
    let connectionError: string | null = $state(null)
    let sessionNotFound = $state(false)
    let sessionLoaded = $state(false)
    let targetRecoveryRequested = false
    // svelte-ignore state_referenced_locally -- keyed by sessionId; initial history is immutable
    let historyToRestore = initialHistory ? [...initialHistory.shells] : []
    // svelte-ignore state_referenced_locally -- keyed by sessionId; initial history is immutable
    const historyActiveIndex = initialHistory?.activeIndex ?? 0
    const restoredChannelIds: string[] = []
    const restoringChannelIds = new Set<string>()
    let pendingHostKey: Extract<
        ServerMessage,
        { type: 'host_key_unknown' }
    > | null = $state(null)
    const tabs: Record<string, SshTerminalTab> = {}
    const pendingOutput = new Map<
        string,
        { chunks: Uint8Array[]; bytes: number }
    >()
    const MAX_PENDING_OUTPUT_BYTES = 1024 * 1024

    let metricsStatus = $state<MetricsStatus>('starting')
    let metricsStatusMessage = $state<string | null>(null)
    let metricsSnapshot = $state<MetricsSnapshot | null>(null)
    let metricsHistory: MetricsSnapshot[] = $state([])
    let metricsLastSampleAt = $state<number | null>(null)

    // svelte-ignore state_referenced_locally -- keyed workspace connection; sessionId is immutable
    const ws = new ReconnectingWebSocket({
        url: `wss://${location.host}/@warpgate/api/web-ssh/sessions/${sessionId}/stream`,
        onOpen: () => {
            if (channelOrder.length === 0) requestNewChannel()
            send({ type: 'start_metrics' })
        },
        onMessage: data =>
            onMessage(JSON.parse(data as string) as ServerMessage),
        onStateChange: (state, attempt) =>
            onConnectionState(sessionId, state, attempt),
        onReconnect: attempts => {
            for (const id of channelOrder) tabs[id]?.writeReconnectMarker()
            onTransportReconnected(sessionId, attempts)
        },
        onReconnectExhausted: () => {
            onReconnectNeeded(sessionId, 'transport')
        },
    })

    function send(msg: ClientMessage) {
        ws.send(JSON.stringify(msg))
    }

    function bytesToBase64(bytes: Uint8Array): string {
        let binary = ''
        const chunkSize = 0x8000
        for (let i = 0; i < bytes.length; i += chunkSize) {
            const chunk = bytes.subarray(i, i + chunkSize)
            binary += String.fromCharCode(...chunk)
        }
        return btoa(binary)
    }

    function requestNewChannel() {
        const size = activeChannelId ? tabs[activeChannelId]?.getSize() : null
        send({
            type: 'open_channel',
            cols: size?.cols ?? 80,
            rows: size?.rows ?? 24,
        })
    }

    function onMessage(msg: ServerMessage) {
        switch (msg.type) {
            case 'connection_state':
                if (msg.state === ConnectionState.Connected) {
                    targetRecoveryRequested = false
                    connectionError = null
                    sessionNotFound = false
                    onError(sessionId, null, false)
                    ws.updateState(ConnectionState.Connected)
                    onTargetConnected(sessionId)
                } else if (msg.state === ConnectionState.Disconnected) {
                    ws.updateState(ConnectionState.TargetOffline)
                    if (!targetRecoveryRequested) {
                        targetRecoveryRequested = true
                        onReconnectNeeded(sessionId, 'target')
                    }
                } else {
                    ws.updateState(ConnectionState.Connecting)
                }
                break
            case 'channel_opened':
                openChannel(msg.channel_id)
                break
            case 'output':
                writeTerminalOutput(
                    msg.channel_id,
                    Uint8Array.from(atob(msg.data), c => c.charCodeAt(0)),
                )
                break
            case 'channel_closed':
            case 'eof': {
                const ch = channels.get(msg.channel_id)
                if (ch) channels.set(msg.channel_id, { ...ch, closed: true })
                break
            }
            case 'exit_status': {
                const ch = channels.get(msg.channel_id)
                if (ch) {
                    writeTerminalOutput(
                        msg.channel_id,
                        Uint8Array.from(
                            `\r\n[Process exited with code ${msg.code}]\r\n`,
                            c => c.charCodeAt(0),
                        ),
                    )
                }
                break
            }
            case 'error':
                ws.updateState(ConnectionState.Error)
                connectionError = msg.message
                onError(sessionId, msg.message, false)
                break
            case 'host_key_unknown':
                pendingHostKey = msg
                break
            case 'metrics_status':
                metricsStatus = msg.state
                metricsStatusMessage = msg.message
                reportMetrics()
                break
            case 'metrics_snapshot':
                metricsStatus = 'available'
                metricsStatusMessage = null
                metricsSnapshot = msg.snapshot
                metricsLastSampleAt = Date.now()
                metricsHistory = [...metricsHistory, msg.snapshot].slice(-60)
                reportMetrics()
                break
        }
    }

    function reportMetrics() {
        onMetrics(sessionId, {
            status: metricsStatus,
            message: metricsStatusMessage,
            snapshot: metricsSnapshot,
            history: metricsHistory,
            lastSampleAt: metricsLastSampleAt,
        })
    }

    function writeTerminalOutput(id: string, data: Uint8Array) {
        const tab = tabs[id]
        if (tab && !restoringChannelIds.has(id)) {
            tab.write(data)
            return
        }

        const backlog = pendingOutput.get(id) ?? { chunks: [], bytes: 0 }
        backlog.chunks.push(data)
        backlog.bytes += data.byteLength
        while (
            backlog.bytes > MAX_PENDING_OUTPUT_BYTES &&
            backlog.chunks.length > 1
        ) {
            const dropped = backlog.chunks.shift()
            if (dropped) backlog.bytes -= dropped.byteLength
        }
        pendingOutput.set(id, backlog)
    }

    function flushTerminalOutput(id: string) {
        const tab = tabs[id]
        const backlog = pendingOutput.get(id)
        if (!tab || !backlog) return
        for (const chunk of backlog.chunks) tab.write(chunk)
        pendingOutput.delete(id)
    }

    async function openChannel(id: string) {
        const history = historyToRestore.shift()
        if (history) restoringChannelIds.add(id)

        channels.set(id, {
            id,
            label: history?.label ?? `Shell ${channelOrder.length + 1}`,
            terminalTitle: history?.terminalTitle,
            closed: false,
        })
        channelOrder = [...channelOrder, id]
        activeChannelId = id

        await tick()
        requestAnimationFrame(() => {
            if (history) {
                tabs[id]?.restoreHistory(history.text)
                restoringChannelIds.delete(id)
                restoredChannelIds.push(id)
            }
            flushTerminalOutput(id)
            tabs[id]?.fit()

            if (historyToRestore.length > 0) {
                requestNewChannel()
            } else if (initialHistory && restoredChannelIds.length > 0) {
                const desired =
                    restoredChannelIds[
                        Math.min(
                            historyActiveIndex,
                            restoredChannelIds.length - 1,
                        )
                    ]
                if (desired && desired !== activeChannelId) {
                    void switchToChannel(desired)
                }
            }
        })
    }

    async function switchToChannel(id: string) {
        activeChannelId = id
        await tick()
        requestAnimationFrame(() => tabs[id]?.fit())
    }

    function closeChannel(id: string) {
        send({ type: 'close_channel', channel_id: id })
        channels.delete(id)
        pendingOutput.delete(id)
        channelOrder = channelOrder.filter(x => x !== id)
        if (activeChannelId === id) {
            activeChannelId = channelOrder[channelOrder.length - 1] ?? null
        }
    }

    function observeResize(node: HTMLElement) {
        const resizeObserver = new ResizeObserver(() => {
            if (active && activeChannelId) tabs[activeChannelId]?.fit()
        })
        resizeObserver.observe(node)
        return {
            destroy() {
                resizeObserver.disconnect()
            },
        }
    }

    export function newShell(): void {
        if (ws.state === ConnectionState.Connected) requestNewChannel()
    }

    export function fit(): void {
        if (!activeChannelId) return
        flushTerminalOutput(activeChannelId)
        tabs[activeChannelId]?.fit()
    }

    export async function snapshotHistory(): Promise<TerminalHistorySnapshot> {
        const shells: TerminalHistorySnapshot['shells'] = []
        for (const id of channelOrder) {
            const channel = channels.get(id)
            const tab = tabs[id]
            if (!channel || !tab) continue
            shells.push({
                label: channel.label,
                terminalTitle: channel.terminalTitle,
                text: await tab.snapshotText(),
            })
        }
        return {
            shells,
            activeIndex: Math.max(
                0,
                activeChannelId ? channelOrder.indexOf(activeChannelId) : 0,
            ),
        }
    }

    export async function disconnect(): Promise<void> {
        ws.close()
        try {
            await api.deleteWebSshSession({ sessionId })
        } catch {
            // The session may already have expired server-side.
        }
    }
    $effect(() => {
        if (!active) return
        requestAnimationFrame(() => fit())
    })

    onMount(async () => {
        try {
            const info = await api.getWebSshSession({ sessionId })
            sessionLoaded = true
            onInfo(sessionId, info)
            onError(sessionId, null, false)
        } catch (e) {
            connectionError =
                e instanceof Error ? e.message : 'Failed to load session info'
            sessionNotFound =
                e instanceof ResponseError && e.response.status === 404
            onError(sessionId, connectionError, sessionNotFound)
            return
        }
        ws.connect()
    })
    onDestroy(() => {
        ws.close()
    })
</script>

<div
    class="connection-pane"
    class:d-none={!active}
    use:observeResize
    style={`background-color: ${theme.background}`}
>
    {#if connectionError && !sessionLoaded}
        <div class="connection-error">
            <InfoBox variant="warning">
                {#if sessionNotFound}
                    Session not found. It may have expired or been closed.
                {:else}
                    {connectionError}
                {/if}
            </InfoBox>
        </div>
    {:else}
        {#if channelOrder.length > 1}
            <div class="shell-strip">
                {#each channelOrder as id (id)}
                    {@const channel = channels.get(id)}
                    {#if channel}
                        <!-- biome-ignore lint/a11y/useSemanticElements: composite tab -->
                        <div
                            class="shell-pill"
                            class:active={id === activeChannelId}
                            role="button"
                            tabindex="0"
                            onclick={() => switchToChannel(id)}
                            onkeydown={e => e.key === 'Enter' && switchToChannel(id)}
                        >
                            <span
                                >{channel.terminalTitle?.trim() || channel.label}</span
                            >
                            <button
                                type="button"
                                class="shell-close"
                                aria-label={`Close ${channel.label}`}
                                onclick={e => {
                                    e.stopPropagation()
                                    closeChannel(id)
                                }}
                            >
                                ×
                            </button>
                        </div>
                    {/if}
                {/each}
            </div>
        {/if}

        <div class="terminal-area">
            {#each channelOrder as id (id)}
                {@const channel = channels.get(id)}
                {#if channel}
                    <SshTerminalTab
                        bind:this={tabs[id]}
                        active={active && id === activeChannelId}
                        {fontSize}
                        {theme}
                        readOnly={ws.state !== ConnectionState.Connected}
                        onInput={data => send({
                            type: 'input',
                            channel_id: id,
                            data: bytesToBase64(data),
                        })}
                        onResize={(cols, rows) => send({
                            type: 'resize',
                            channel_id: id,
                            cols,
                            rows,
                        })}
                        onTitleChange={title => {
                            channels.set(id, {
                                ...channel,
                                terminalTitle: title.trim() || undefined,
                            })
                        }}
                    />
                {/if}
            {/each}
        </div>
    {/if}
</div>

{#if pendingHostKey}
    <Modal isOpen={true} backdrop="static" keyboard={false}>
        <ModalBody>
            <div class="mb-3">
                There is currently no trusted {pendingHostKey.key_type} key for
                the SSH server at {pendingHostKey.host}:{pendingHostKey.port}.
                Trust this key?
            </div>
            <code>{pendingHostKey.key_type} {pendingHostKey.key_base64}</code>
        </ModalBody>
        <ModalFooter>
            <Button
                color="danger"
                class="modal-button"
                onclick={() => {
                    send({ type: 'reject_host_key' })
                    pendingHostKey = null
                    disconnect()
                }}
            >
                Reject and disconnect
            </Button>
            <Button
                color="primary"
                class="modal-button"
                onclick={() => {
                    send({ type: 'accept_host_key' })
                    pendingHostKey = null
                }}
            >
                Accept and connect
            </Button>
        </ModalFooter>
    </Modal>
{/if}

<style lang="scss">
    .connection-pane {
        position: absolute;
        inset: 0;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .connection-error {
        padding: 1rem;
    }
    .terminal-area {
        position: relative;
        min-height: 0;
        flex: 1 1 auto;
        overflow: hidden;
    }

    .shell-strip {
        flex: 0 0 auto;
        display: flex;
        gap: 4px;
        padding: 4px 8px;
        overflow-x: auto;
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        background: rgba(0, 0, 0, 0.18);
    }

    .shell-pill {
        display: flex;
        align-items: center;
        gap: 6px;
        max-width: 240px;
        padding: 3px 7px 3px 9px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 5px;
        color: rgba(255, 255, 255, 0.58);
        background: rgba(255, 255, 255, 0.035);
        font-size: 0.72rem;
        cursor: pointer;
    }
    .shell-pill.active {
        color: rgba(255, 255, 255, 0.94);
        border-color: rgba(88, 166, 255, 0.38);
        background: rgba(88, 166, 255, 0.12);
    }

    .shell-pill span {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .shell-close {
        border: 0;
        padding: 0 2px;
        color: inherit;
        background: transparent;
        line-height: 1;
        opacity: 0.65;
    }

    .shell-close:hover {
        opacity: 1;
    }
</style>
