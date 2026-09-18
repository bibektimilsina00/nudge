import * as rc from "../rive_advanced.mjs";
export interface KeyboardInteractionsParams {
    canvas: HTMLCanvasElement;
    stateMachine: rc.StateMachineInstance;
    /**
     * Whether this canvas has focus nodes that should participate in tab traversal.
     * When true, Tab/Shift+Tab will be intercepted and routed to the Rive focus manager.
     * focusNext() returning false means no more traversable nodes — tab is released to the page.
     */
    hasFocusNodes: boolean;
    /**
     * Optional accessibility overlay that should be treated as part of this Rive
     * instance's focus domain. This is lazy because the overlay may be created
     * after keyboard listeners are registered.
     */
    getOverlayElement?: () => HTMLElement | null;
}
/**
 * Tracks the relationship between DOM focus inside this Rive focus domain (canvas or semantic overlay)
 * and Rive's internal focus for the current focus session.
 *
 * NotFocused   — DOM focus left the domain, Rive released focus internally, or Tab walked
 *                off the end of the tree. Keyboard input isn't ours, so Tab is ignored and
 *                reaches the next page element.
 * EntryPending — DOM focus is inside the domain but Rive holds no node yet, so the next Tab
 *                enters the focus tree. Set by pointer focus on the canvas, by assistive technology (AT) focus landing
 *                in the overlay, and by keyboard focus whose entry attempt found no eligible node.
 * RiveFocused  — a Rive node holds focus. Tab/Shift+Tab route to the Rive focus manager and stay
 *                inside the domain until either Rive reports focus ended (pollFocusState) or
 *                focusNext()/focusPrevious() returns false at the edge of the tree.
 *
 * Keyboard focus on the canvas enters the tree immediately: onCanvasFocus infers direction from
 * where focus came from and goes straight to RiveFocused when a node accepts.
 */
export declare enum FocusSessionState {
    NotFocused = "notFocused",
    EntryPending = "entryPending",
    RiveFocused = "riveFocused"
}
/**
 * Manages keyboard and DOM focus interactions for Rive's focus domain (<canvas> or semantic overlay).
 * Because keyboard events can apply on either part of the domain, we need to track what events we should
 * handle/intercept, and when to release focus back to the page outside of the domain.
 *
 * Tracks the canvas focus session state (focusSessionState) and routes
 * Tab/Shift+Tab to the Rive state machine's focus manager. Exposes shared
 * state as properties so the Rive render loop can read them directly.
 */
export declare class KeyboardInteractions {
    focusSessionState: FocusSessionState;
    private canvas;
    private mainSm;
    private hasFocusNodes;
    /** Cached callback that returns the accessibility overlay element once created. */
    private getOverlayElement?;
    /** Whether the canvas currently has browser DOM focus. */
    private canvasHasFocus;
    /** After Tab exits the last Rive node, ignore keydowns until focus re-enters the focus domain. */
    private focusDomainReleased;
    /** Overlay element currently wired with focusin/keydown listeners, if any. */
    private currentOverlayElement;
    /** Canvas parent (or document) watched for focusin to attach overlay listeners lazily. */
    private focusDomainHost;
    constructor({ canvas, stateMachine, hasFocusNodes, getOverlayElement, }: KeyboardInteractionsParams);
    /**
     * Set the FocusSessionState. Useful for invoking a Rive "blur" without actually blurring from the <canvas>. This
     * helps put the DOM focus state on the canvas rather than the <body>, so the user doesn't lose the spot in page navigation
     *
     * @param state FocusSessionState enum
     */
    setFocusSessionState(state: FocusSessionState): void;
    /**
     * Called by pollFocusState on the Rive instance when it observes hasFocus=true. Rive acquired
     * focus internally (e.g. via a listener action or state transition) without a DOM focus event,
     * so mark the session RiveFocused. This cannot resurrect a session that a DOM blur ended,
     * because onCanvasBlur clears Rive's focus alongside it.
     */
    notifyRiveFocused(): void;
    /**
     * Handles the canvas gaining browser focus. The behavior differs based on how focus was gained -
     *
     * Pointer-driven focus: the canvas now has focus but Rive holds nothing yet, so we move to EntryPending — this lets the
     * next Tab enter the focus tree even when the focus is pointer-driven
     *
     * Keyboard-driven focus: we enter the Rive focus tree immediately once canvas gains focus.
     * The direction is inferred from where focus came from: an element before the canvas in DOM order
     * means a forward Tab (focusNext), one after means a Shift+Tab (focusPrevious). :focus-visible
     * gates this so a click doesn't yank Rive focus to the first node on the focus event itself.
     */
    onCanvasFocus: (event: FocusEvent) => void;
    /**
     * Marks internal state that the canvas has lost DOM focus. Do not actually clear
     * Rive focus though if:
     * 1. DOM focus is still within Rive domain (i.e., semantic overlay)
     * 2. Document just lost focus (i.e. tab switching)
     *
     * When we're not in either of those buckets, it's safe to call `clearFocus()` on the SMI.
     */
    onCanvasBlur: (event: FocusEvent) => void;
    /**
     * Assistive technology (AT) focus landing inside the overlay is DOM focus inside the Rive focus domain, so open a
     * session even when no Rive node holds focus yet. shouldRiveHandleKeyEvent treats NotFocused
     * as authoritative, so without this the overlay's Tab keydowns reach onKeyDown and get dropped
     * at that gate — the browser would move focus out of Rive instead of to the next focus node.
     */
    private onOverlayFocusIn;
    /** Overlay listeners attach lazily, so the first focusin only ever lands here. */
    private onFocusDomainHostFocusIn;
    onKeyDown: (event: KeyboardEvent) => void;
    /**
     * Determine if Rive should handle keyboard input. If session state is `NotFocused` - no.
     * DOM focus stays parked on the canvas after Rive releases focus internally, and that Tab
     * has to reach the page rather than re-enter the tree.
     *
     * Otherwise, the event still has to belong to Rive:
     * 1. If the current DOM focus is in Rive domain (canvas or semantic overlay)
     * 2. If the target for the key input is for the semantic overlay, or the canvas
     */
    private shouldRiveHandleKeyEvent;
    /**
     * The Rive focus domain: the DOM that counts as "inside" Rive for focus purposes — today the
     * canvas itself OR the accessibility overlay subtree. Anything added later belongs here, so
     * session bookkeeping and keydown routing pick it up for free.
     */
    private isInFocusDomain;
    /** Overlay only (excludes the canvas) — the accessibility overlay subtree. */
    private isInOverlay;
    private syncOverlayListener;
    /**
     * Whether the canvas currently matches :focus-visible — the browser's heuristic for keyboard-
     * (vs pointer-) driven focus. For older browser versions that don't support this selector, return false
     * so that we don't incorrectly assume pointer vs keyboard focus. Next tab would enter the focus tree in those edge cases.
     */
    private isKeyboardDrivenFocus;
    private cameFromBeforeCanvas;
    cleanup(): void;
}
