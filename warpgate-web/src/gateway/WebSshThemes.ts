export type TerminalThemeName =
    | 'tokyo-night'
    | 'catppuccin-mocha'
    | 'dracula'
    | 'nord'
    | 'warpgate'

export interface TerminalTheme {
    label: string
    foreground: string
    background: string
    cursor: string
    selectionBackground: string
    colors: readonly string[]
}

export const DEFAULT_TERMINAL_THEME: TerminalThemeName = 'tokyo-night'

export const TERMINAL_THEMES: Record<TerminalThemeName, TerminalTheme> = {
    'tokyo-night': {
        label: 'Tokyo Night',
        foreground: '#c0caf5',
        background: '#1a1b26',
        cursor: '#c0caf5',
        selectionBackground: '#33467c',
        colors: [
            '#15161e',
            '#f7768e',
            '#9ece6a',
            '#e0af68',
            '#7aa2f7',
            '#bb9af7',
            '#7dcfff',
            '#a9b1d6',
            '#414868',
            '#f7768e',
            '#9ece6a',
            '#e0af68',
            '#7aa2f7',
            '#bb9af7',
            '#7dcfff',
            '#c0caf5',
        ],
    },
    'catppuccin-mocha': {
        label: 'Catppuccin Mocha',
        foreground: '#cdd6f4',
        background: '#1e1e2e',
        cursor: '#f5e0dc',
        selectionBackground: '#45475a',
        colors: [
            '#45475a',
            '#f38ba8',
            '#a6e3a1',
            '#f9e2af',
            '#89b4fa',
            '#f5c2e7',
            '#94e2d5',
            '#bac2de',
            '#585b70',
            '#f38ba8',
            '#a6e3a1',
            '#f9e2af',
            '#89b4fa',
            '#f5c2e7',
            '#94e2d5',
            '#cdd6f4',
        ],
    },
    dracula: {
        label: 'Dracula',
        foreground: '#f8f8f2',
        background: '#282a36',
        cursor: '#f8f8f2',
        selectionBackground: '#44475a',
        colors: [
            '#21222c',
            '#ff5555',
            '#50fa7b',
            '#f1fa8c',
            '#6272a4',
            '#ff79c6',
            '#8be9fd',
            '#f8f8f2',
            '#6272a4',
            '#ff6e6e',
            '#69ff94',
            '#ffffa5',
            '#d6acff',
            '#ff92df',
            '#a4ffff',
            '#ffffff',
        ],
    },
    nord: {
        label: 'Nord',
        foreground: '#d8dee9',
        background: '#2e3440',
        cursor: '#d8dee9',
        selectionBackground: '#434c5e',
        colors: [
            '#3b4252',
            '#bf616a',
            '#a3be8c',
            '#ebcb8b',
            '#81a1c1',
            '#b48ead',
            '#88c0d0',
            '#e5e9f0',
            '#4c566a',
            '#bf616a',
            '#a3be8c',
            '#ebcb8b',
            '#81a1c1',
            '#b48ead',
            '#8fbcbb',
            '#eceff4',
        ],
    },
    warpgate: {
        label: 'Warpgate Original',
        foreground: '#cacaca',
        background: '#171717',
        cursor: '#bbbbbb',
        selectionBackground: '#3b3b3b',
        colors: [
            '#000000',
            '#ff615a',
            '#b1e969',
            '#ebd99c',
            '#5da9f6',
            '#e86aff',
            '#82fff7',
            '#dedacf',
            '#313131',
            '#f58c80',
            '#ddf88f',
            '#eee5b2',
            '#a5c7ff',
            '#ddaaff',
            '#b7fff9',
            '#ffffff',
        ],
    },
}
export function isTerminalThemeName(
    value: string | null,
): value is TerminalThemeName {
    return value !== null && value in TERMINAL_THEMES
}

export function toXtermTheme(theme: TerminalTheme) {
    return {
        foreground: theme.foreground,
        background: theme.background,
        cursor: theme.cursor,
        selectionBackground: theme.selectionBackground,
        black: theme.colors[0],
        red: theme.colors[1],
        green: theme.colors[2],
        yellow: theme.colors[3],
        blue: theme.colors[4],
        magenta: theme.colors[5],
        cyan: theme.colors[6],
        white: theme.colors[7],
        brightBlack: theme.colors[8],
        brightRed: theme.colors[9],
        brightGreen: theme.colors[10],
        brightYellow: theme.colors[11],
        brightBlue: theme.colors[12],
        brightMagenta: theme.colors[13],
        brightCyan: theme.colors[14],
        brightWhite: theme.colors[15],
    }
}
