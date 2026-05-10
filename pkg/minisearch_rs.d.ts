/* tslint:disable */
/* eslint-disable */

/**
 * JS-facing search engine. Owns an [`Engine`] internally.
 */
export class JsEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add a document. Returns a JS error if `id` is a duplicate.
     */
    addDocument(id: string, text: string): void;
    /**
     * Number of indexed documents.
     */
    docCount(): number;
    /**
     * Restore an engine from a previously-saved index byte blob.
     *
     * Returns a `JsValue` error if decoding fails.
     */
    static fromBytes(bytes: Uint8Array): JsEngine;
    /**
     * Create an empty engine with default BM25 config.
     */
    constructor();
    /**
     * Run a query, return up to `top_k` ranked hits as a JS array of
     * `{ doc_id, score, snippet?: { text, highlights: [[s,e], …] } }`.
     *
     * `corpus` is an optional `id -> original text` map (`Map<string, string>`
     * from JS) used to build snippets. Pass `null`/`undefined` to skip
     * snippet generation.
     */
    search(query: string, top_k: number, corpus: any): any;
    /**
     * Number of unique terms (vocabulary size).
     */
    termCount(): number;
    /**
     * Serialize the index to a `Uint8Array` so callers can persist it.
     */
    toBytes(): Uint8Array;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_jsengine_free: (a: number, b: number) => void;
    readonly jsengine_addDocument: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly jsengine_docCount: (a: number) => number;
    readonly jsengine_fromBytes: (a: number, b: number, c: number) => void;
    readonly jsengine_new: () => number;
    readonly jsengine_search: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly jsengine_termCount: (a: number) => number;
    readonly jsengine_toBytes: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
