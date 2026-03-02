// react Reference - React UI Library
// package.json: "react": "^18.2.0"
// Usage: import React, { useState, useEffect } from 'react';

import React from 'react';
import { useState, useEffect, useCallback, useMemo, useRef, useContext, useReducer } from 'react';

// ============================================================================
// HOOKS
// ============================================================================

function useState<S>(initialState: S | (() => S)): [S, (value: S | ((prev: S) => S)) => void];
function useEffect(effect: () => void | (() => void), deps?: ReadonlyArray<any>): void;
function useCallback<T extends Function>(callback: T, deps: ReadonlyArray<any>): T;
function useMemo<T>(factory: () => T, deps: ReadonlyArray<any>): T;
function useRef<T>(initialValue: T): { current: T };
function useContext<T>(context: React.Context<T>): T;
function useReducer<S, A>(reducer: (state: S, action: A) => S, initialState: S): [S, (action: A) => void];

// ============================================================================
// COMPONENT TYPES
// ============================================================================

interface FC<P = {}> {
    (props: P): React.ReactElement | null;
    displayName?: string;
}

type ReactNode = React.ReactElement | string | number | boolean | null | undefined;
type ReactElement = { type: any; props: any; key: string | null };
type JSXElement = React.ReactElement;

// ============================================================================
// CONTEXT
// ============================================================================

function createContext<T>(defaultValue: T): React.Context<T>;

interface Context<T> {
    Provider: React.Provider<T>;
    Consumer: React.Consumer<T>;
    displayName?: string;
}

// ============================================================================
// REFS
// ============================================================================

function createRef<T>(): React.RefObject<T>;
function forwardRef<T, P = {}>(render: (props: P, ref: React.Ref<T>) => React.ReactElement | null): React.ForwardRefExoticComponent<P & React.RefAttributes<T>>;

// ============================================================================
// MEMO
// ============================================================================

function memo<P>(component: React.FC<P>, areEqual?: (prev: P, next: P) => boolean): React.FC<P>;

// ============================================================================
// COMMON PATTERNS
// ============================================================================

// State management
function ExampleComponent(): JSX.Element {
    const [count, setCount] = useState(0);
    const [name, setName] = useState<string>("");

    useEffect(() => {
        document.title = `Count: ${count}`;
        return () => { /* cleanup */ };
    }, [count]);

    return <div>{count}</div>;
}

// Context usage
const ThemeContext = createContext<string>("light");

function ThemedComponent(): JSX.Element {
    const theme = useContext(ThemeContext);
    return <div className={theme}>Themed</div>;
}
