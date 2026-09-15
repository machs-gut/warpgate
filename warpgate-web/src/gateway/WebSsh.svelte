<script lang="ts">
    import {
        faChartLine,
        faChevronLeft,
        faChevronRight,
        faGear,
        faMinus,
        faPlus,
        faServer,
        faTerminal,
        faTimes,
    } from '@fortawesome/free-solid-svg-icons'
    import {
        Button,
        Dropdown,
        DropdownItem,
        DropdownMenu,
        DropdownToggle,
        Input,
        Modal,
        ModalBody,
        ModalFooter,
    } from '@sveltestrap/sveltestrap'
    import ConnectionInstructions from 'common/ConnectionInstructions.svelte'
    import { handleReauthError } from 'common/reauth'
    import { onMount, tick } from 'svelte'
    import Fa from 'svelte-fa'
    import { loadTheme } from 'theme'
    import {
        api,
        TargetKind,
        type TargetSnapshot,
        type WebSshSessionInfo,
    } from './lib/api'
    import { ConnectionState } from './lib/ReconnectingWebSocket.svelte'
    import { reloadServerInfo, serverInfo } from './lib/store'
    import WebSshConnection from './WebSshConnection.svelte'
    import WebSshMetrics from './WebSshMetrics.svelte'
    import {
        DEFAULT_TERMINAL_THEME,
        isTerminalThemeName,
        TERMINAL_THEMES,
        type TerminalThemeName,
    } from './WebSshThemes'
    import type { MetricsViewState } from './WebSshTypes'

    interface Props {
        params: { sessionId: string }
    }
    let { params }: Props = $props()

    interface WorkspaceConnection {
        sessionId: string
        targetId: string | null
        targetName: string
        targetKind: TargetKind
        state: ConnectionState
        attempt: number
        metrics: MetricsViewState
        error: string | null
        notFound: boolean
    }

    interface TargetGroupView {
        id: string
        name: string
        targets: TargetSnapshot[]
    }

    const emptyMetrics = (): MetricsViewState => ({
        status: 'starting',
        message: null,
        snapshot: null,
        history: [],
        lastSampleAt: null,
    })

    // svelte-ignore state_referenced_locally
    const initialSessionId = params.sessionId
    let connections: WorkspaceConnection[] = $state([
        {
            sessionId: initialSessionId,
            targetId: null,
            targetName: 'Loading…',
            targetKind: TargetKind.Ssh,
            state: ConnectionState.Connecting,
            attempt: 0,
            metrics: emptyMetrics(),
            error: null,
            notFound: false,
        },
    ])
    let activeSessionId: string | null = $state(initialSessionId)
    const panes: Record<string, WebSshConnection> = {}

    let sshTargets: TargetSnapshot[] = $state([])
    let targetsLoading = $state(true)
    let targetLoadError = $state<string | null>(null)
    let connectError = $state<string | null>(null)
    let targetSearch = $state('')
    let selectedTargetIds: string[] = $state([])
    let pendingTargetIds: string[] = $state([])
    let reconnectingSessionIds: string[] = $state([])
    let collapsedGroupIds: string[] = $state([])

    const storedSidebar = localStorage.getItem('warpgateWebSSHSidebarOpen')
    const storedMetricsRail = localStorage.getItem(
        'warpgateWebSSHMetricsRailOpen',
    )
    const SIDEBAR_WIDTH_DEFAULT = 246
    const SIDEBAR_WIDTH_MIN = 180
    const SIDEBAR_WIDTH_MAX = 520
    const storedSidebarWidth = Number.parseInt(
        localStorage.getItem('warpgateWebSSHSidebarWidth') ?? '',
        10,
    )
    let sidebarOpen = $state(storedSidebar !== 'false')
    let metricsRailOpen = $state(storedMetricsRail !== 'false')
    let sidebarWidth = $state(
        Number.isFinite(storedSidebarWidth)
            ? Math.min(
                  SIDEBAR_WIDTH_MAX,
                  Math.max(SIDEBAR_WIDTH_MIN, storedSidebarWidth),
              )
            : SIDEBAR_WIDTH_DEFAULT,
    )
    let sidebarResizing = $state(false)
    let sidebarFitFrame: number | null = null

    const FONT_SIZE_MIN = 8
    const FONT_SIZE_MAX = 32
    const FONT_SIZE_STEP = 1
    let fontSize = $state(
        parseInt(localStorage.warpgateWebSSHFontSize ?? '14', 10),
    )
    const storedTerminalTheme = localStorage.getItem('warpgateWebSSHTheme')
    let terminalThemeName = $state<TerminalThemeName>(
        isTerminalThemeName(storedTerminalTheme)
            ? storedTerminalTheme
            : DEFAULT_TERMINAL_THEME,
    )
    let terminalTheme = $derived(TERMINAL_THEMES[terminalThemeName])
    const terminalThemeOptions = Object.entries(TERMINAL_THEMES) as Array<
        [TerminalThemeName, (typeof TERMINAL_THEMES)[TerminalThemeName]]
    >
    let menuOpen = $state(false)
    let showInstructions = $state(false)

    let activeConnection = $derived(
        activeSessionId
            ? (connections.find(
                  connection => connection.sessionId === activeSessionId,
              ) ?? null)
            : null,
    )
    let connectedTargetIds = $derived(
        connections
            .map(connection => connection.targetId)
            .filter((id): id is string => !!id),
    )
    let filteredTargets = $derived.by(() => {
        const search = targetSearch.trim().toLowerCase()
        if (!search) return sshTargets
        return sshTargets.filter(target =>
            `${target.group?.name ?? ''} ${target.name} ${target.description}`
                .toLowerCase()
                .includes(search),
        )
    })
    let targetGroups = $derived.by<TargetGroupView[]>(() => {
        const groups = new Map<string, TargetGroupView>()
        for (const target of filteredTargets) {
            const id = target.group?.id ?? '__ungrouped__'
            const group = groups.get(id) ?? {
                id,
                name: target.group?.name ?? 'Ungrouped',
                targets: [],
            }
            group.targets.push(target)
            groups.set(id, group)
        }
        const collator = new Intl.Collator(undefined, {
            numeric: true,
            sensitivity: 'base',
        })
        return [...groups.values()]
            .map(group => ({
                ...group,
                targets: [...group.targets].sort((a, b) =>
                    collator.compare(a.name, b.name),
                ),
            }))
            .sort((a, b) => collator.compare(a.name, b.name))
    })

    $effect(() => {
        localStorage.warpgateWebSSHFontSize = String(fontSize)
    })
    $effect(() => {
        localStorage.warpgateWebSSHTheme = terminalThemeName
    })
    $effect(() => {
        localStorage.warpgateWebSSHSidebarOpen = String(sidebarOpen)
    })
    $effect(() => {
        localStorage.warpgateWebSSHMetricsRailOpen = String(metricsRailOpen)
    })

    function zoomIn() {
        fontSize = Math.min(FONT_SIZE_MAX, fontSize + FONT_SIZE_STEP)
    }
    function zoomOut() {
        fontSize = Math.max(FONT_SIZE_MIN, fontSize - FONT_SIZE_STEP)
    }

    function clampSidebarWidth(width: number) {
        return Math.min(
            SIDEBAR_WIDTH_MAX,
            Math.max(SIDEBAR_WIDTH_MIN, Math.round(width)),
        )
    }

    function fitActivePane() {
        if (sidebarFitFrame !== null) {
            cancelAnimationFrame(sidebarFitFrame)
        }
        sidebarFitFrame = requestAnimationFrame(() => {
            sidebarFitFrame = null
            if (activeSessionId) panes[activeSessionId]?.fit()
        })
    }

    function persistSidebarWidth() {
        localStorage.warpgateWebSSHSidebarWidth = String(sidebarWidth)
    }

    function startSidebarResize(event: PointerEvent) {
        if (window.innerWidth < 1100) return
        event.preventDefault()

        const startX = event.clientX
        const startWidth = sidebarWidth
        sidebarResizing = true
        document.body.classList.add('webssh-sidebar-resizing')

        const onPointerMove = (moveEvent: PointerEvent) => {
            sidebarWidth = clampSidebarWidth(
                startWidth + moveEvent.clientX - startX,
            )
            fitActivePane()
        }
        const finish = () => {
            window.removeEventListener('pointermove', onPointerMove)
            window.removeEventListener('pointerup', finish)
            window.removeEventListener('pointercancel', finish)
            document.body.classList.remove('webssh-sidebar-resizing')
            sidebarResizing = false
            persistSidebarWidth()
            fitActivePane()
        }

        window.addEventListener('pointermove', onPointerMove)
        window.addEventListener('pointerup', finish)
        window.addEventListener('pointercancel', finish)
    }

    function resizeSidebarFromKeyboard(event: KeyboardEvent) {
        if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
        event.preventDefault()
        const delta = event.key === 'ArrowLeft' ? -16 : 16
        sidebarWidth = clampSidebarWidth(sidebarWidth + delta)
        persistSidebarWidth()
        fitActivePane()
    }

    function connectionBySessionId(sessionId: string) {
        return connections.find(
            connection => connection.sessionId === sessionId,
        )
    }

    function updateConnection(
        sessionId: string,
        patch: Partial<WorkspaceConnection>,
    ) {
        const index = connections.findIndex(
            connection => connection.sessionId === sessionId,
        )
        if (index < 0) return
        const current = connections[index]
        if (!current) return
        connections[index] = { ...current, ...patch }
    }

    function connectionForTarget(targetId: string) {
        return connections.find(connection => connection.targetId === targetId)
    }

    function reconcileTargetId(sessionId: string) {
        const connection = connectionBySessionId(sessionId)
        if (!connection || connection.targetId || !connection.targetName) return
        const matches = sshTargets.filter(
            target =>
                target.name === connection.targetName &&
                target.kind === connection.targetKind,
        )
        const [match] = matches
        if (matches.length === 1 && match) {
            updateConnection(sessionId, { targetId: match.id })
        }
    }

    function onInfo(sessionId: string, info: WebSshSessionInfo) {
        updateConnection(sessionId, {
            targetName: info.targetName,
            targetKind: info.targetKind,
            error: null,
            notFound: false,
        })
        reconcileTargetId(sessionId)
    }

    function onConnectionState(
        sessionId: string,
        state: ConnectionState,
        attempt: number,
    ) {
        updateConnection(sessionId, { state, attempt })
    }

    function onMetrics(sessionId: string, metrics: MetricsViewState) {
        updateConnection(sessionId, { metrics })
    }

    function onError(
        sessionId: string,
        message: string | null,
        notFound: boolean,
    ) {
        updateConnection(sessionId, { error: message, notFound })
    }

    async function switchSession(sessionId: string) {
        activeSessionId = sessionId
        await tick()
        requestAnimationFrame(() => panes[sessionId]?.fit())
    }

    function reconnectTargetId(connection: WorkspaceConnection) {
        if (connection.targetId) return connection.targetId
        const matches = sshTargets.filter(
            target =>
                target.name === connection.targetName &&
                target.kind === connection.targetKind,
        )
        return matches.length === 1 ? (matches[0]?.id ?? null) : null
    }

    async function reconnectSession(sessionId: string) {
        if (reconnectingSessionIds.includes(sessionId)) return
        const connection = connectionBySessionId(sessionId)
        if (!connection) return

        const targetId = reconnectTargetId(connection)
        if (!targetId) {
            updateConnection(sessionId, {
                state: ConnectionState.Error,
                error: 'Unable to identify the SSH target for reconnect',
                notFound: false,
            })
            return
        }

        reconnectingSessionIds = [...reconnectingSessionIds, sessionId]
        connectError = null
        try {
            const { sessionId: newSessionId } = await api.createWebSshSession({
                createWebSshSessionBody: { targetId },
            })

            const currentIndex = connections.findIndex(
                item => item.sessionId === sessionId,
            )
            if (currentIndex < 0) {
                try {
                    await api.deleteWebSshSession({ sessionId: newSessionId })
                } catch {
                    // The replacement session may already have expired.
                }
                return
            }

            await panes[sessionId]?.disconnect()
            delete panes[sessionId]

            connections[currentIndex] = {
                ...connection,
                sessionId: newSessionId,
                targetId,
                state: ConnectionState.Connecting,
                attempt: 0,
                metrics: emptyMetrics(),
                error: null,
                notFound: false,
            }
            if (activeSessionId === sessionId) {
                activeSessionId = newSessionId
            }
            await tick()
            requestAnimationFrame(() => panes[newSessionId]?.fit())
        } catch (err) {
            if (!(await handleReauthError(err))) {
                connectError =
                    err instanceof Error
                        ? err.message
                        : 'Failed to reconnect target'
            }
        } finally {
            reconnectingSessionIds = reconnectingSessionIds.filter(
                id => id !== sessionId,
            )
        }
    }

    async function openServerPicker() {
        sidebarOpen = true
        await tick()
        requestAnimationFrame(() => {
            const search = document.querySelector<HTMLInputElement>(
                '.target-search input',
            )
            search?.focus()
            search?.select()
        })
    }

    function addConnection(
        sessionId: string,
        target: TargetSnapshot,
    ): WorkspaceConnection {
        const connection: WorkspaceConnection = {
            sessionId,
            targetId: target.id,
            targetName: target.name,
            targetKind: target.kind,
            state: ConnectionState.Connecting,
            attempt: 0,
            metrics: emptyMetrics(),
            error: null,
            notFound: false,
        }
        connections = [...connections, connection]
        return connection
    }

    async function connectTarget(target: TargetSnapshot) {
        const existing = connectionForTarget(target.id)
        if (existing) {
            await switchSession(existing.sessionId)
            return existing
        }
        if (pendingTargetIds.includes(target.id)) return null

        pendingTargetIds = [...pendingTargetIds, target.id]
        connectError = null
        try {
            const { sessionId } = await api.createWebSshSession({
                createWebSshSessionBody: { targetId: target.id },
            })
            const connection = addConnection(sessionId, target)
            await switchSession(sessionId)
            return connection
        } catch (err) {
            if (!(await handleReauthError(err))) {
                connectError =
                    err instanceof Error
                        ? err.message
                        : 'Failed to connect target'
            }
            return null
        } finally {
            pendingTargetIds = pendingTargetIds.filter(id => id !== target.id)
        }
    }

    function toggleSelected(targetId: string) {
        selectedTargetIds = selectedTargetIds.includes(targetId)
            ? selectedTargetIds.filter(id => id !== targetId)
            : [...selectedTargetIds, targetId]
    }

    async function activateTarget(target: TargetSnapshot) {
        const connection = connectionForTarget(target.id)
        if (connection) {
            await switchSession(connection.sessionId)
            return
        }
        if (pendingTargetIds.includes(target.id)) return
        selectedTargetIds = selectedTargetIds.filter(id => id !== target.id)
        await connectTarget(target)
    }

    async function connectSelected() {
        const selected = sshTargets.filter(
            target =>
                selectedTargetIds.includes(target.id) &&
                !connectedTargetIds.includes(target.id),
        )
        for (const target of selected) {
            await connectTarget(target)
        }
        selectedTargetIds = []
    }

    async function closeSession(sessionId: string) {
        await panes[sessionId]?.disconnect()
        delete panes[sessionId]
        connections = connections.filter(
            connection => connection.sessionId !== sessionId,
        )
        if (activeSessionId === sessionId) {
            const nextSessionId =
                connections[connections.length - 1]?.sessionId ?? null
            activeSessionId = nextSessionId
            if (nextSessionId) {
                await tick()
                requestAnimationFrame(() => panes[nextSessionId]?.fit())
            }
        }
    }

    async function disconnectAll() {
        for (const connection of [...connections]) {
            await panes[connection.sessionId]?.disconnect()
            delete panes[connection.sessionId]
        }
        connections = []
        activeSessionId = null
    }
    function toggleGroup(groupId: string) {
        collapsedGroupIds = collapsedGroupIds.includes(groupId)
            ? collapsedGroupIds.filter(id => id !== groupId)
            : [...collapsedGroupIds, groupId]
    }

    function newShell() {
        if (activeSessionId) panes[activeSessionId]?.newShell()
    }

    function connectionStateLabel(connection: WorkspaceConnection | null) {
        if (!connection) return 'No active session'
        if (connection.error)
            return connection.notFound ? 'Session expired' : 'Error'
        if (
            connection.state === ConnectionState.Connecting &&
            connection.attempt > 0
        ) {
            return `${connection.state} · attempt ${connection.attempt}`
        }
        return connection.state
    }

    function stateClass(connection: WorkspaceConnection) {
        if (connection.error || connection.state === ConnectionState.Error)
            return 'error'
        if (connection.state === ConnectionState.Connected) return 'connected'
        return 'connecting'
    }

    async function loadTargets() {
        targetsLoading = true
        targetLoadError = null
        try {
            const targets = await api.getTargets({ search: '' })
            sshTargets = targets.filter(
                target => target.kind === TargetKind.Ssh,
            )
            for (const connection of connections)
                reconcileTargetId(connection.sessionId)
        } catch (err) {
            targetLoadError =
                err instanceof Error
                    ? err.message
                    : 'Failed to load SSH targets'
        } finally {
            targetsLoading = false
        }
    }

    onMount(() => {
        reloadServerInfo()
        loadTargets()
        if (window.innerWidth < 1100) {
            if (storedSidebar === null) sidebarOpen = false
            if (storedMetricsRail === null) metricsRailOpen = false
        }
    })

    const originalTitle = document.title
    let windowTitle = $derived(
        activeConnection
            ? `${activeConnection.targetName} - ${originalTitle}`
            : originalTitle,
    )
    $effect(() => {
        document.title = windowTitle
    })

    $effect(() => {
        void loadTheme(terminalTheme.appearance)
    })
</script>

<div
    class="webssh-workspace"
    class:light-workspace={terminalTheme.appearance === 'light'}
    class:sidebar-resizing={sidebarResizing}
    style={`background-color: ${terminalTheme.background}; --webssh-sidebar-width: ${sidebarWidth}px`}
>
    <div class="workspace-topbar">
        <button
            type="button"
            class="icon-button"
            class:active={sidebarOpen}
            aria-label={sidebarOpen ? 'Hide servers' : 'Show servers'}
            title={sidebarOpen ? 'Hide servers' : 'Show servers'}
            onclick={() => sidebarOpen = !sidebarOpen}
        >
            <Fa icon={faServer} />
        </button>

        <div class="session-tabs">
            {#each connections as connection (connection.sessionId)}
                {@const sessionId = connection.sessionId}
                <!-- biome-ignore lint/a11y/useSemanticElements: composite tab -->
                <div
                    class="session-tab"
                    class:active={sessionId === activeSessionId}
                    role="button"
                    tabindex="0"
                    title={`Double-click to reconnect ${connection.targetName}`}
                    onclick={() => switchSession(sessionId)}
                    ondblclick={() => reconnectSession(sessionId)}
                    onkeydown={e =>
                            e.key === 'Enter' && switchSession(sessionId)}
                >
                    <span
                        class="connection-dot {stateClass(connection)}"
                    ></span>
                    <span class="session-name">{connection.targetName}</span>
                    <button
                        type="button"
                        class="tab-close"
                        aria-label={`Disconnect ${connection.targetName}`}
                        onclick={e => {
                                e.stopPropagation()
                                closeSession(sessionId)
                            }}
                        ondblclick={e => e.stopPropagation()}
                    >
                        <Fa icon={faTimes} />
                    </button>
                </div>
            {/each}

            <button
                type="button"
                class="top-action"
                title="Add server"
                aria-label="Add server"
                onclick={openServerPicker}
            >
                <Fa icon={faPlus} />
            </button>
            <button
                type="button"
                class="top-action"
                disabled={!activeConnection || activeConnection.state !== ConnectionState.Connected}
                title="New shell on current server"
                aria-label="New shell on current server"
                onclick={newShell}
            >
                <Fa icon={faTerminal} />+
            </button>
        </div>

        <button
            type="button"
            class="icon-button"
            class:active={metricsRailOpen}
            aria-label={metricsRailOpen ? 'Hide metrics' : 'Show metrics'}
            title={metricsRailOpen ? 'Hide metrics' : 'Show metrics'}
            onclick={() => metricsRailOpen = !metricsRailOpen}
        >
            <Fa icon={faChartLine} />
        </button>
    </div>

    <div class="workspace-body">
        {#if sidebarOpen}
            <aside class="targets-panel">
                <div class="panel-header">
                    <div>
                        <div class="panel-kicker">WORKSPACE</div>
                        <div class="panel-title">Servers</div>
                    </div>
                    <button
                        type="button"
                        class="panel-collapse"
                        aria-label="Hide servers"
                        onclick={() => sidebarOpen = false}
                    >
                        <Fa icon={faChevronLeft} />
                    </button>
                </div>

                <div class="target-search">
                    <Input
                        type="search"
                        placeholder="Search servers"
                        bind:value={targetSearch}
                        aria-label="Search SSH targets"
                    />
                </div>

                {#if connectError}
                    <div class="connect-error">{connectError}</div>
                {/if}

                <div class="targets-list">
                    {#if targetsLoading}
                        <div class="panel-empty">Loading targets…</div>
                    {:else if targetLoadError}
                        <div class="panel-error">{targetLoadError}</div>
                    {:else if targetGroups.length === 0}
                        <div class="panel-empty">No SSH targets found</div>
                    {:else}
                        {#each targetGroups as group (group.id)}
                            <section class="target-group">
                                <button
                                    type="button"
                                    class="group-header"
                                    onclick={() => toggleGroup(group.id)}
                                >
                                    <span class="group-chevron">
                                        {collapsedGroupIds.includes(group.id) ? '›' : '⌄'}
                                    </span>
                                    <span class="group-name">{group.name}</span>
                                    <span class="group-count"
                                        >{group.targets.length}</span
                                    >
                                </button>

                                {#if !collapsedGroupIds.includes(group.id)}
                                    <div class="group-targets">
                                        {#each group.targets as target (target.id)}
                                            {@const connected = connectedTargetIds.includes(target.id)}
                                            {@const pending = pendingTargetIds.includes(target.id)}
                                            {@const selected = selectedTargetIds.includes(target.id)}
                                            {@const connection = connectionForTarget(target.id)}
                                            <!-- biome-ignore lint/a11y/useSemanticElements: composite target selector -->
                                            <div
                                                class="target-row"
                                                class:connected
                                                class:selected
                                                role="button"
                                                tabindex="0"
                                                onclick={() => {
                                                    if (connection) {
                                                        switchSession(connection.sessionId)
                                                    } else if (!pending) {
                                                        toggleSelected(target.id)
                                                    }
                                                }}
                                                ondblclick={() =>
                                                    activateTarget(target)}
                                                onkeydown={e => {
                                                    if (e.key !== 'Enter') return
                                                    if (connection) switchSession(connection.sessionId)
                                                    else if (!pending) toggleSelected(target.id)
                                                }}
                                            >
                                                <input
                                                    type="checkbox"
                                                    checked={connected || selected}
                                                    disabled={connected || pending}
                                                    aria-label={`Select ${target.name}`}
                                                    onclick={e => e.stopPropagation()}
                                                    onchange={() => toggleSelected(target.id)}
                                                >
                                                <span class="target-main">
                                                    <span class="target-name"
                                                        >{target.name}</span
                                                    >
                                                    {#if target.description}
                                                        <span
                                                            class="target-description"
                                                            >{target.description}</span
                                                        >
                                                    {/if}
                                                </span>
                                                {#if pending}
                                                    <span
                                                        class="target-state pending"
                                                        >…</span
                                                    >
                                                {:else if connected}
                                                    <span
                                                        class="target-state connected"
                                                        >●</span
                                                    >
                                                {/if}
                                            </div>
                                        {/each}
                                    </div>
                                {/if}
                            </section>
                        {/each}
                    {/if}
                </div>

                <div class="targets-footer">
                    <Button
                        color="primary"
                        size="sm"
                        disabled={selectedTargetIds.length === 0}
                        onclick={connectSelected}
                    >
                        Connect{selectedTargetIds.length > 0
                            ? ` ${selectedTargetIds.length}`
                            : ''}
                    </Button>
                    <span>{connectedTargetIds.length} connected</span>
                </div>

                <button
                    type="button"
                    class="sidebar-resizer"
                    class:active={sidebarResizing}
                    aria-label="Resize servers panel"
                    title="Drag to resize servers panel; use left/right arrow keys for fine adjustment"
                    onpointerdown={startSidebarResize}
                    onkeydown={resizeSidebarFromKeyboard}
                ></button>
            </aside>
        {/if}

        <main class="terminal-stack">
            {#each connections as connection (connection.sessionId)}
                {@const sessionId = connection.sessionId}
                <WebSshConnection
                    bind:this={panes[sessionId]}
                    {sessionId}
                    active={sessionId === activeSessionId}
                    {fontSize}
                    theme={terminalTheme}
                    {onInfo}
                    {onConnectionState}
                    {onMetrics}
                    {onError}
                />
            {/each}

            {#if connections.length === 0}
                <div class="workspace-empty">
                    <Fa icon={faServer} size="2x" />
                    <strong>No server connected</strong>
                    <span>Select SSH targets from the Servers panel.</span>
                    <Button
                        color="primary"
                        size="sm"
                        onclick={() => sidebarOpen = true}
                    >
                        Choose servers
                    </Button>
                </div>
            {/if}
        </main>

        {#if metricsRailOpen}
            <aside class="metrics-panel">
                <div class="metrics-panel-header">
                    <div>
                        <div class="panel-kicker">CURRENT SERVER</div>
                        <div class="panel-title">
                            {activeConnection?.targetName ?? 'Metrics'}
                        </div>
                    </div>
                    <button
                        type="button"
                        class="panel-collapse"
                        aria-label="Hide metrics"
                        onclick={() => metricsRailOpen = false}
                    >
                        <Fa icon={faChevronRight} />
                    </button>
                </div>
                {#if activeConnection}
                    <WebSshMetrics
                        layout="rail"
                        status={activeConnection.metrics.status}
                        message={activeConnection.metrics.message}
                        snapshot={activeConnection.metrics.snapshot}
                        history={activeConnection.metrics.history}
                        lastSampleAt={activeConnection.metrics.lastSampleAt}
                    />
                {:else}
                    <div class="panel-empty">
                        Connect a server to see metrics.
                    </div>
                {/if}
            </aside>
        {/if}
    </div>

    <div class="workspace-footer">
        <div class="footer-status">
            {#if activeConnection}
                <span
                    class="connection-dot {stateClass(activeConnection)}"
                ></span>
                <strong>{activeConnection.targetName}</strong>
                <span>{connectionStateLabel(activeConnection)}</span>
            {:else}
                <span>No active session</span>
            {/if}
        </div>

        <div class="footer-actions">
            {#if activeConnection}
                <Button
                    color="danger"
                    size="sm"
                    onclick={() =>
                        activeConnection && closeSession(activeConnection.sessionId)}
                >
                    Disconnect
                </Button>
            {/if}

            <Dropdown bind:isOpen={menuOpen}>
                <DropdownToggle color="secondary" size="sm" caret={false}>
                    <Fa icon={faGear} />
                </DropdownToggle>
                <DropdownMenu end>
                    <div
                        class="dropdown-item disabled font-size-row d-flex align-items-center gap-2"
                    >
                        <button
                            type="button"
                            class="btn btn-sm btn-secondary"
                            disabled={fontSize <= FONT_SIZE_MIN}
                            onclick={() => { zoomOut(); menuOpen = true }}
                            aria-label="Zoom out"
                        >
                            <Fa icon={faMinus} />
                        </button>
                        <span class="text-nowrap ms-auto me-auto"
                            >{fontSize}px</span
                        >
                        <button
                            type="button"
                            class="btn btn-sm btn-secondary"
                            disabled={fontSize >= FONT_SIZE_MAX}
                            onclick={() => { zoomIn(); menuOpen = true }}
                            aria-label="Zoom in"
                        >
                            <Fa icon={faPlus} />
                        </button>
                    </div>
                    <DropdownItem divider />
                    <div class="dropdown-header">Terminal theme</div>
                    {#each terminalThemeOptions as [name, option]}
                        <DropdownItem
                            onclick={() => {
                                terminalThemeName = name
                                menuOpen = false
                            }}
                        >
                            <span class="theme-check">
                                {terminalThemeName === name ? '✓' : ''}
                            </span>
                            {option.label}
                        </DropdownItem>
                    {/each}
                    <DropdownItem divider />
                    <DropdownItem onclick={() => sidebarOpen = !sidebarOpen}>
                        {sidebarOpen ? 'Hide' : 'Show'}
                        servers panel
                    </DropdownItem>
                    <DropdownItem
                        onclick={() => metricsRailOpen = !metricsRailOpen}
                    >
                        {metricsRailOpen ? 'Hide' : 'Show'}
                        metrics panel
                    </DropdownItem>
                    {#if activeConnection}
                        <DropdownItem divider />
                        <DropdownItem
                            onclick={() => {
                                showInstructions = true
                                menuOpen = false
                            }}
                        >
                            Connect from your machine
                        </DropdownItem>
                    {/if}
                    {#if connections.length > 1}
                        <DropdownItem divider />
                        <DropdownItem
                            onclick={() => {
                                disconnectAll()
                                menuOpen = false
                            }}
                        >
                            Disconnect all
                        </DropdownItem>
                    {/if}
                </DropdownMenu>
            </Dropdown>
        </div>
    </div>
</div>

{#if activeConnection}
    <Modal
        isOpen={showInstructions}
        toggle={() => showInstructions = false}
        size="lg"
    >
        <ModalBody>
            <ConnectionInstructions
                targetName={activeConnection.targetName}
                targetKind={activeConnection.targetKind}
                username={$serverInfo?.username}
            />
        </ModalBody>
        <ModalFooter>
            <Button color="secondary" onclick={() => showInstructions = false}>
                Close
            </Button>
        </ModalFooter>
    </Modal>
{/if}

<style lang="scss">
    :global(body) {
        margin: 0;
        overflow: hidden;
    }

    .webssh-workspace {
        width: 100vw;
        height: 100vh;
        display: flex;
        flex-direction: column;
        color: rgba(255, 255, 255, 0.88);
        overflow: hidden;
    }

    .workspace-topbar,
    .workspace-footer {
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        min-height: 42px;
        padding: 5px 8px;
        background: rgba(0, 0, 0, 0.58);
        border-color: rgba(255, 255, 255, 0.07);
        border-style: solid;
        border-width: 0 0 1px;
    }

    .workspace-footer {
        min-height: 38px;
        justify-content: space-between;
        border-width: 1px 0 0;
    }

    .workspace-body {
        position: relative;
        min-height: 0;
        flex: 1 1 auto;
        display: flex;
        overflow: hidden;
    }

    .icon-button,
    .top-action,
    .panel-collapse,
    .tab-close {
        border: 0;
        color: rgba(255, 255, 255, 0.58);
        background: transparent;
    }

    .icon-button {
        width: 32px;
        height: 32px;
        flex: 0 0 auto;
        border-radius: 6px;
    }

    .icon-button:hover,
    .icon-button.active,
    .top-action:hover:not(:disabled),
    .panel-collapse:hover {
        color: rgba(255, 255, 255, 0.94);
        background: rgba(255, 255, 255, 0.08);
    }

    .session-tabs {
        min-width: 0;
        flex: 1 1 auto;
        display: flex;
        align-items: stretch;
        gap: 5px;
        margin: 0 7px;
        overflow-x: auto;
        scrollbar-width: thin;
    }

    .session-tab {
        min-width: 120px;
        max-width: 240px;
        height: 32px;
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        gap: 7px;
        padding: 0 5px 0 10px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 6px;
        color: rgba(255, 255, 255, 0.62);
        background: rgba(255, 255, 255, 0.035);
        cursor: pointer;
    }

    .session-tab.active {
        color: rgba(255, 255, 255, 0.96);
        border-color: rgba(88, 166, 255, 0.44);
        background: rgba(88, 166, 255, 0.13);
    }

    .session-name {
        min-width: 0;
        flex: 1 1 auto;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 0.78rem;
        font-weight: 600;
    }

    .connection-dot {
        width: 7px;
        height: 7px;
        flex: 0 0 auto;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.3);
    }
    .connection-dot.connected {
        background: #7ee787;
        box-shadow: 0 0 7px rgba(126, 231, 135, 0.42);
    }

    .connection-dot.connecting {
        background: #d29922;
    }

    .connection-dot.error {
        background: #ff7b72;
    }

    .tab-close {
        width: 22px;
        height: 22px;
        padding: 0;
        border-radius: 4px;
        opacity: 0.58;
    }

    .tab-close:hover {
        opacity: 1;
        background: rgba(255, 255, 255, 0.09);
    }

    .top-action {
        min-width: 32px;
        height: 32px;
        flex: 0 0 auto;
        border-radius: 6px;
        font-size: 0.72rem;
    }
    .top-action:disabled {
        opacity: 0.3;
    }

    .targets-panel,
    .metrics-panel {
        min-height: 0;
        flex: 0 0 auto;
        display: flex;
        flex-direction: column;
        background: rgba(10, 12, 18, 0.94);
        border-color: rgba(255, 255, 255, 0.07);
        z-index: 10;
    }

    .targets-panel {
        position: relative;
        width: var(--webssh-sidebar-width, 246px);
        border-right: 1px solid rgba(255, 255, 255, 0.07);
    }

    .sidebar-resizer {
        position: absolute;
        top: 0;
        right: -4px;
        bottom: 0;
        width: 8px;
        z-index: 20;
        padding: 0;
        border: 0;
        outline: 0;
        background: transparent;
        cursor: col-resize;
        touch-action: none;
    }

    .sidebar-resizer::after {
        content: '';
        position: absolute;
        top: 0;
        bottom: 0;
        left: 3px;
        width: 2px;
        background: transparent;
        transition: background 120ms ease;
    }

    .sidebar-resizer:hover::after,
    .sidebar-resizer:focus-visible::after,
    .sidebar-resizer.active::after {
        background: rgba(88, 166, 255, 0.72);
    }

    .sidebar-resizing {
        cursor: col-resize;
        user-select: none;
    }

    :global(body.webssh-sidebar-resizing) {
        cursor: col-resize !important;
        user-select: none !important;
    }

    .metrics-panel {
        width: 244px;
        border-left: 1px solid rgba(255, 255, 255, 0.07);
    }

    .panel-header,
    .metrics-panel-header {
        min-height: 52px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 8px 10px 7px 12px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    }
    .panel-kicker {
        color: rgba(255, 255, 255, 0.34);
        font-size: 0.58rem;
        font-weight: 800;
        letter-spacing: 0.08em;
    }

    .panel-title {
        max-width: 185px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: rgba(255, 255, 255, 0.9);
        font-size: 0.9rem;
        font-weight: 700;
    }

    .panel-collapse {
        width: 28px;
        height: 28px;
        flex: 0 0 auto;
        border-radius: 5px;
    }

    .target-search {
        padding: 8px 9px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }

    .target-search :global(input) {
        height: 32px;
        border-color: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.86);
        background: rgba(255, 255, 255, 0.045);
    }

    .targets-list {
        min-height: 0;
        flex: 1 1 auto;
        overflow-y: auto;
        padding: 5px 0 8px;
    }

    .group-header {
        width: 100%;
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 6px 10px;
        border: 0;
        color: rgba(255, 255, 255, 0.58);
        background: transparent;
        text-align: left;
        font-size: 0.72rem;
        font-weight: 700;
    }

    .group-header:hover {
        color: rgba(255, 255, 255, 0.88);
        background: rgba(255, 255, 255, 0.035);
    }

    .group-chevron {
        width: 10px;
        font-size: 0.9rem;
    }

    .group-name {
        min-width: 0;
        flex: 1 1 auto;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .group-count {
        color: rgba(255, 255, 255, 0.3);
        font-size: 0.65rem;
    }

    .group-targets {
        padding: 0 5px 4px;
    }

    .target-row {
        min-height: 34px;
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 1px 0;
        padding: 5px 7px 5px 12px;
        border-radius: 5px;
        color: rgba(255, 255, 255, 0.68);
        cursor: pointer;
    }

    .target-row:hover,
    .target-row.selected {
        color: rgba(255, 255, 255, 0.94);
        background: rgba(88, 166, 255, 0.09);
    }

    .target-row.connected {
        background: rgba(126, 231, 135, 0.045);
    }

    .target-row input {
        width: 13px;
        height: 13px;
        flex: 0 0 auto;
        accent-color: #58a6ff;
    }

    .target-main {
        min-width: 0;
        flex: 1 1 auto;
        display: flex;
        flex-direction: column;
    }

    .target-name,
    .target-description {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .target-name {
        font-size: 0.76rem;
        font-weight: 560;
    }

    .target-description {
        color: rgba(255, 255, 255, 0.34);
        font-size: 0.64rem;
    }

    .target-state.connected {
        color: #7ee787;
        font-size: 0.62rem;
    }

    .target-state.pending {
        color: #d29922;
    }

    .targets-footer {
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        min-height: 46px;
        padding: 7px 9px;
        border-top: 1px solid rgba(255, 255, 255, 0.06);
        color: rgba(255, 255, 255, 0.4);
        font-size: 0.68rem;
    }

    .panel-empty,
    .panel-error {
        padding: 14px 12px;
        color: rgba(255, 255, 255, 0.42);
        font-size: 0.72rem;
    }

    .panel-error {
        color: #ff7b72;
    }

    .connect-error {
        margin: 7px 9px 0;
        padding: 6px 8px;
        border: 1px solid rgba(248, 81, 73, 0.28);
        border-radius: 5px;
        color: #ff7b72;
        background: rgba(248, 81, 73, 0.07);
        font-size: 0.68rem;
    }

    .terminal-stack {
        position: relative;
        min-width: 0;
        min-height: 0;
        flex: 1 1 auto;
        overflow: hidden;
    }

    .workspace-empty {
        position: absolute;
        inset: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 10px;
        color: rgba(255, 255, 255, 0.42);
        text-align: center;
    }

    .workspace-empty strong {
        color: rgba(255, 255, 255, 0.78);
    }

    .workspace-empty span {
        font-size: 0.78rem;
    }

    .metrics-panel :global(.metrics-shell) {
        flex: 1 1 auto;
        min-height: 0;
    }

    .footer-status,
    .footer-actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .footer-status {
        min-width: 0;
        color: rgba(255, 255, 255, 0.42);
        font-size: 0.7rem;
    }

    .footer-status strong {
        max-width: 240px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: rgba(255, 255, 255, 0.72);
    }

    .font-size-row {
        min-width: 220px;
        padding: 0.25rem 1rem;
        pointer-events: none;
    }

    .font-size-row button {
        pointer-events: initial;
    }

    .theme-check {
        display: inline-block;
        width: 1.25rem;
    }

    .light-workspace {
        color: #24292f;
    }

    .light-workspace .workspace-topbar,
    .light-workspace .workspace-footer {
        background: rgba(246, 248, 250, 0.98);
        border-color: #d0d7de;
    }

    .light-workspace .icon-button,
    .light-workspace .top-action,
    .light-workspace .panel-collapse,
    .light-workspace .tab-close {
        color: #57606a;
    }

    .light-workspace .icon-button:hover,
    .light-workspace .icon-button.active,
    .light-workspace .top-action:hover:not(:disabled),
    .light-workspace .panel-collapse:hover,
    .light-workspace .tab-close:hover {
        color: #24292f;
        background: rgba(9, 105, 218, 0.08);
    }

    .light-workspace .session-tab {
        color: #57606a;
        border-color: #d0d7de;
        background: rgba(255, 255, 255, 0.84);
    }

    .light-workspace .session-tab.active {
        color: #24292f;
        border-color: rgba(9, 105, 218, 0.55);
        background: #ddf4ff;
    }

    .light-workspace .connection-dot {
        background: #8c959f;
    }

    .light-workspace .connection-dot.connected,
    .light-workspace .target-state.connected {
        background: #1a7f37;
        color: #1a7f37;
        box-shadow: 0 0 7px rgba(26, 127, 55, 0.24);
    }

    .light-workspace .connection-dot.connecting,
    .light-workspace .target-state.pending {
        background: #9a6700;
        color: #9a6700;
    }

    .light-workspace .connection-dot.error,
    .light-workspace .panel-error {
        background: #cf222e;
        color: #cf222e;
    }

    .light-workspace .targets-panel,
    .light-workspace .metrics-panel {
        background: rgba(255, 255, 255, 0.97);
        border-color: #d0d7de;
    }

    .light-workspace .targets-panel {
        border-right-color: #d0d7de;
    }

    .light-workspace .sidebar-resizer:hover::after,
    .light-workspace .sidebar-resizer:focus-visible::after,
    .light-workspace .sidebar-resizer.active::after {
        background: rgba(9, 105, 218, 0.72);
    }

    .light-workspace .metrics-panel {
        border-left-color: #d0d7de;
    }

    .light-workspace .panel-header,
    .light-workspace .metrics-panel-header,
    .light-workspace .target-search,
    .light-workspace .targets-footer {
        border-color: rgba(27, 31, 36, 0.12);
    }

    .light-workspace .panel-kicker,
    .light-workspace .target-description,
    .light-workspace .targets-footer,
    .light-workspace .panel-empty,
    .light-workspace .footer-status {
        color: #6e7781;
    }

    .light-workspace .panel-title,
    .light-workspace .workspace-empty strong,
    .light-workspace .footer-status strong {
        color: #24292f;
    }

    .light-workspace .target-search :global(input) {
        color: #24292f;
        border-color: #d0d7de;
        background: #ffffff;
    }

    .light-workspace .group-header,
    .light-workspace .target-row,
    .light-workspace .workspace-empty {
        color: #57606a;
    }

    .light-workspace .group-header:hover,
    .light-workspace .target-row:hover,
    .light-workspace .target-row.selected {
        color: #24292f;
        background: rgba(9, 105, 218, 0.08);
    }

    .light-workspace .group-count {
        color: #8c959f;
    }

    .light-workspace .target-row.connected {
        background: rgba(26, 127, 55, 0.07);
    }

    .light-workspace .connect-error {
        color: #cf222e;
        border-color: rgba(207, 34, 46, 0.28);
        background: rgba(207, 34, 46, 0.07);
    }

    .light-workspace :global(.shell-strip) {
        border-bottom-color: #d0d7de;
        background: rgba(246, 248, 250, 0.86);
    }

    .light-workspace :global(.shell-pill) {
        color: #57606a;
        border-color: #d0d7de;
        background: #ffffff;
    }

    .light-workspace :global(.shell-pill.active) {
        color: #24292f;
        border-color: rgba(9, 105, 218, 0.48);
        background: #ddf4ff;
    }

    .light-workspace .metrics-panel :global(.metrics-shell) {
        color: #24292f;
    }

    .light-workspace .metrics-panel :global(.status-dot) {
        background: #8c959f;
    }

    .light-workspace .metrics-panel :global(.status-dot.available) {
        background: #1a7f37;
        box-shadow: 0 0 7px rgba(26, 127, 55, 0.24);
    }

    .light-workspace .metrics-panel :global(.status-dot.stale) {
        background: #9a6700;
        box-shadow: 0 0 7px rgba(154, 103, 0, 0.2);
    }

    .light-workspace .metrics-panel :global(.status-label),
    .light-workspace .metrics-panel :global(.interface-label),
    .light-workspace .metrics-panel :global(.metrics-empty),
    .light-workspace .metrics-panel :global(.metric-sub),
    .light-workspace .metrics-panel :global(.metric-title) {
        color: #6e7781;
    }

    .light-workspace .metrics-panel :global(.metric-card) {
        border-color: #d0d7de;
        background: rgba(255, 255, 255, 0.9);
    }

    .light-workspace .metrics-panel :global(.metric-card.warning) {
        border-color: rgba(154, 103, 0, 0.5);
        background: rgba(154, 103, 0, 0.08);
    }

    .light-workspace .metrics-panel :global(.metric-card.critical) {
        border-color: rgba(207, 34, 46, 0.5);
        background: rgba(207, 34, 46, 0.08);
    }

    .light-workspace .metrics-panel :global(.metric-card.warning .metric-value),
    .light-workspace .metrics-panel :global(.metric-card.warning .metric-title) {
        color: #9a6700;
    }

    .light-workspace .metrics-panel :global(.metric-card.critical .metric-value),
    .light-workspace .metrics-panel :global(.metric-card.critical .metric-title) {
        color: #cf222e;
    }

    .light-workspace .metrics-panel :global(.rail .status-block) {
        border-bottom-color: rgba(27, 31, 36, 0.12);
    }

    @media (max-width: 1100px) {
        .targets-panel,
        .metrics-panel {
            position: absolute;
            top: 0;
            bottom: 0;
            z-index: 30;
            box-shadow: 0 0 28px rgba(0, 0, 0, 0.38);
        }

        .targets-panel {
            left: 0;
            width: min(86vw, var(--webssh-sidebar-width, 246px));
        }

        .metrics-panel {
            right: 0;
        }

        .sidebar-resizer {
            display: none;
        }
    }

    @media (max-width: 700px) {
        .targets-panel,
        .metrics-panel {
            width: min(86vw, 280px);
        }

        .session-tab {
            min-width: 105px;
        }

        .footer-status span:last-child {
            display: none;
        }
    }
</style>
