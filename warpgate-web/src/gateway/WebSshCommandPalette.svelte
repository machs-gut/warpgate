<script lang="ts">
    import { onMount, tick } from 'svelte'

    interface CommandItem {
        id: string
        label: string
        detail?: string
        shortcut?: string
        keywords?: string
        disabled?: boolean
        run: () => void | Promise<void>
    }

    interface Props {
        items: CommandItem[]
        light: boolean
        onClose: () => void
    }

    let { items, light, onClose }: Props = $props()
    let query = $state('')
    let selectedIndex = $state(0)
    let inputElement: HTMLInputElement | null = null

    let filteredItems = $derived.by(() => {
        const needle = query.trim().toLowerCase()
        if (!needle) return items
        return items.filter(item =>
            `${item.label} ${item.detail ?? ''} ${item.keywords ?? ''}`
                .toLowerCase()
                .includes(needle),
        )
    })

    function moveSelection(delta: number) {
        if (filteredItems.length === 0) return
        let next = selectedIndex
        for (let count = 0; count < filteredItems.length; count += 1) {
            next = (next + delta + filteredItems.length) % filteredItems.length
            if (!filteredItems[next]?.disabled) {
                selectedIndex = next
                return
            }
        }
    }

    function invoke(item: CommandItem | undefined) {
        if (!item || item.disabled) return
        onClose()
        void item.run()
    }

    function onKeydown(event: KeyboardEvent) {
        if (event.key === 'Escape') {
            event.preventDefault()
            onClose()
            return
        }
        if (event.key === 'ArrowDown') {
            event.preventDefault()
            moveSelection(1)
            return
        }
        if (event.key === 'ArrowUp') {
            event.preventDefault()
            moveSelection(-1)
            return
        }
        if (event.key === 'Enter') {
            event.preventDefault()
            invoke(filteredItems[selectedIndex])
        }
    }

    onMount(async () => {
        await tick()
        inputElement?.focus()
        inputElement?.select()
    })
</script>

<div class="command-palette-backdrop" class:light>
    <dialog
        open
        class="command-palette"
        aria-modal="true"
        aria-label="Web SSH command palette"
        onkeydown={onKeydown}
    >
        <div class="palette-search-row">
            <span class="palette-prompt">›</span>
            <input
                bind:this={inputElement}
                bind:value={query}
                oninput={() => selectedIndex = 0}
                type="search"
                placeholder="Type a command or server name…"
                aria-label="Filter commands"
                autocomplete="off"
                spellcheck="false"
            >
            <kbd>Esc</kbd>
            <button
                type="button"
                class="palette-close"
                aria-label="Close command palette"
                title="Close command palette"
                onclick={onClose}
            >
                ×
            </button>
        </div>

        <div class="palette-list" role="listbox" aria-label="Commands">
            {#if filteredItems.length === 0}
                <div class="palette-empty">No matching commands</div>
            {:else}
                {#each filteredItems as item, index (item.id)}
                    <button
                        type="button"
                        class="palette-item"
                        class:selected={index === selectedIndex}
                        disabled={item.disabled}
                        role="option"
                        aria-selected={index === selectedIndex}
                        onpointerenter={() => selectedIndex = index}
                        onclick={() => invoke(item)}
                    >
                        <span class="palette-copy">
                            <span class="palette-label">{item.label}</span>
                            {#if item.detail}
                                <span class="palette-detail"
                                    >{item.detail}</span
                                >
                            {/if}
                        </span>
                        {#if item.shortcut}
                            <kbd>{item.shortcut}</kbd>
                        {/if}
                    </button>
                {/each}
            {/if}
        </div>

        <div class="palette-footer">
            <span><kbd>↑</kbd><kbd>↓</kbd> navigate</span>
            <span><kbd>Enter</kbd> run</span>
            <span><kbd>Esc</kbd> close</span>
            <span class="palette-count">{filteredItems.length} commands</span>
        </div>
    </dialog>
</div>

<style lang="scss">
    .command-palette-backdrop {
        position: fixed;
        inset: 0;
        z-index: 2000;
        display: flex;
        align-items: flex-start;
        justify-content: center;
        padding-top: min(13vh, 110px);
        background: rgba(0, 0, 0, 0.52);
        backdrop-filter: blur(3px);
    }

    .command-palette {
        position: relative;
        inset: auto;
        margin: 0;
        width: min(720px, calc(100vw - 32px));
        max-height: min(70vh, 640px);
        display: flex;
        flex-direction: column;
        overflow: hidden;
        border: 1px solid rgba(255, 255, 255, 0.13);
        border-radius: 10px;
        color: rgba(255, 255, 255, 0.92);
        background: #151821;
        box-shadow: 0 20px 70px rgba(0, 0, 0, 0.52);
    }

    .palette-search-row {
        display: flex;
        align-items: center;
        gap: 9px;
        min-height: 48px;
        padding: 7px 10px 7px 14px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    }

    .palette-prompt {
        color: #58a6ff;
        font-size: 1.15rem;
        font-weight: 800;
    }

    .palette-search-row input {
        min-width: 0;
        flex: 1 1 auto;
        border: 0;
        outline: 0;
        color: inherit;
        background: transparent;
        font-size: 0.92rem;
    }

    .palette-search-row input::placeholder {
        color: rgba(255, 255, 255, 0.35);
    }

    .palette-close {
        width: 26px;
        height: 26px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 0;
        border: 0;
        border-radius: 5px;
        color: rgba(255, 255, 255, 0.48);
        background: transparent;
        font-size: 1rem;
        line-height: 1;
    }

    .palette-close:hover {
        color: rgba(255, 255, 255, 0.9);
        background: rgba(255, 255, 255, 0.08);
    }

    kbd {
        flex: 0 0 auto;
        padding: 2px 6px;
        border: 1px solid rgba(255, 255, 255, 0.13);
        border-bottom-color: rgba(255, 255, 255, 0.2);
        border-radius: 4px;
        color: rgba(255, 255, 255, 0.54);
        background: rgba(255, 255, 255, 0.055);
        box-shadow: none;
        font-family: inherit;
        font-size: 0.64rem;
        font-weight: 650;
    }

    .palette-list {
        min-height: 0;
        overflow-y: auto;
        padding: 6px;
    }

    .palette-item {
        width: 100%;
        min-height: 42px;
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 6px 9px;
        border: 0;
        border-radius: 6px;
        color: rgba(255, 255, 255, 0.79);
        background: transparent;
        text-align: left;
    }

    .palette-item.selected:not(:disabled) {
        color: rgba(255, 255, 255, 0.98);
        background: rgba(88, 166, 255, 0.16);
    }

    .palette-item:disabled {
        opacity: 0.38;
    }

    .palette-copy {
        min-width: 0;
        flex: 1 1 auto;
        display: flex;
        flex-direction: column;
        gap: 1px;
    }

    .palette-label,
    .palette-detail {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .palette-label {
        font-size: 0.79rem;
        font-weight: 600;
    }

    .palette-detail {
        color: rgba(255, 255, 255, 0.38);
        font-size: 0.65rem;
    }

    .palette-empty {
        padding: 30px 16px;
        color: rgba(255, 255, 255, 0.38);
        text-align: center;
        font-size: 0.75rem;
    }

    .palette-footer {
        display: flex;
        align-items: center;
        gap: 12px;
        min-height: 34px;
        padding: 6px 10px;
        border-top: 1px solid rgba(255, 255, 255, 0.07);
        color: rgba(255, 255, 255, 0.38);
        font-size: 0.63rem;
    }

    .palette-footer span {
        display: flex;
        align-items: center;
        gap: 3px;
    }

    .palette-count {
        margin-left: auto;
    }

    .command-palette-backdrop.light {
        background: rgba(31, 35, 40, 0.24);
    }

    .light .command-palette {
        color: #24292f;
        border-color: #d0d7de;
        background: #ffffff;
        box-shadow: 0 20px 70px rgba(31, 35, 40, 0.22);
    }

    .light .palette-search-row,
    .light .palette-footer {
        border-color: #d8dee4;
    }

    .light .palette-search-row input::placeholder,
    .light .palette-detail,
    .light .palette-empty,
    .light .palette-footer {
        color: #6e7781;
    }

    .light .palette-item {
        color: #57606a;
    }

    .light .palette-item.selected:not(:disabled) {
        color: #24292f;
        background: #ddf4ff;
    }

    .light .palette-close {
        color: #6e7781;
    }

    .light .palette-close:hover {
        color: #24292f;
        background: rgba(9, 105, 218, 0.08);
    }

    .light kbd {
        color: #57606a;
        border-color: #d0d7de;
        background: #f6f8fa;
    }

    @media (max-width: 700px) {
        .command-palette-backdrop {
            padding-top: 8vh;
        }

        .palette-footer span:not(.palette-count) {
            display: none;
        }
    }
</style>
