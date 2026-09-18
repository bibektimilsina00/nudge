import { Layout, RiveParameters, Rive as Rive$1, RiveFileParameters, RiveFile, ViewModelInstance, ViewModelInstanceArtboard, decodeImage, decodeFont, StateMachineInput, ViewModel, Bounds } from '@rive-app/canvas';
export * from '@rive-app/canvas';
import React, { ComponentProps, RefCallback, MutableRefObject } from 'react';

interface RiveProps {
    /**
     * URL of the Rive asset, or path to where the public asset is stored.
     */
    src: string;
    /**
     * Artboard to render from the Rive asset.
     * Defaults to the first artboard created.
     */
    artboard?: string;
    /**
     * Name of the state machine to play.
     */
    stateMachine?: string;
    /**
     * Specify a starting animation to play.
     *
     * @deprecated Use the `stateMachine` prop to play a state machine instead.
     * Support for starting playback with named animations will be removed in a
     * future major version. See
     * {@link https://rive.app/docs/editor/state-machine/state-machine} for more on
     * setting up state machines in your Rive file.
     */
    animations?: string | string[];
    /**
     * Specify a starting state machine to play.
     *
     * @deprecated Use `stateMachine` with a single state machine name instead.
     * Support for playing multiple state machines at once will be removed in a
     * future major version.
     */
    stateMachines?: string | string[];
    /**
     * Specify a starting Layout object to set Fill and Alignment for the drawing surface. See docs at https://rive.app/docs/runtimes/react/layouts for more on layout configuration.
     */
    layout?: Layout;
    /**
     * For `@rive-app/react-webgl2`, share one WebGL context across every canvas
     * on the page instead of giving this canvas its own.
     *
     * **We recommend leaving this unset**, which picks the right value for you:
     * - `true` normally, so multiple Rive graphics on a page share one context.
     * - `false` when `enableGPUCanvas` is on, because GPU Canvas needs a context
     *   of its own for each `<canvas>`.
     *
     * Setting it explicitly always wins. Setting it to `true` alongside
     * `enableGPUCanvas` means GPU Canvas content will not draw.
     */
    useOffscreenRenderer?: boolean;
    /**
     * @experimental This API is early and may encounter breaking behavior change without a major version bump
     *
     * Render GPU Canvas content, which draws through the runtime's deferred
     * renderer. False by default.
     *
     * IMPORTANT: Turning this on makes `useOffscreenRenderer` default to `false` for this
     * component if it was not specified, since a GPU Canvas session records for a single `<canvas>` context. Highly recommended to
     * only set `true` on this property for graphics that use GPU Canvas content, and leave
     * `enableGPUCanvas` unset, or set to `false` for other graphics on the page.
     *
     * Each of these takes its own WebGL context, and browsers cap how many a page may
     * have (the limit varies by browser), so enabling it on too many at once can throw.
     */
    enableGPUCanvas?: boolean;
    /**
     * Specify whether to disable Rive listeners on the canvas, thus preventing any event listeners to be attached to the canvas element
     */
    shouldDisableRiveListeners?: boolean;
    /**
     * Specify whether to resize the canvas to its container automatically
     */
    shouldResizeCanvasToContainer?: boolean;
    /**
     * Enable Rive Events to be handled by the runtime. This means any special Rive Event may have
     * functionality that can be invoked implicitly when detected.
     *
     * For example, if during the render loop an OpenUrlEvent is detected, the
     * browser may try to open the specified URL in the payload.
     *
     * This flag is false by default to prevent any unwanted behaviors from taking place.
     * This means any special Rive Event will have to be handled manually by subscribing to
     * EventType.RiveEvent
     *
     * @deprecated Rive Events are deprecated and will be removed in a future
     * major version: please use data binding instead. See
     * {@link https://rive.app/docs/runtimes/web/rive-events} for how to migrate.
     */
    automaticallyHandleEvents?: boolean;
}
declare const Rive: ({ src, artboard, stateMachine, animations, stateMachines, layout, useOffscreenRenderer, enableGPUCanvas, shouldDisableRiveListeners, shouldResizeCanvasToContainer, automaticallyHandleEvents, children, ...rest }: RiveProps & ComponentProps<'canvas'>) => React.JSX.Element;

type UseRiveParameters = (Partial<Omit<RiveParameters, 'canvas'>> & {
    onRiveReady?: (rive: Rive$1) => void;
}) | null;
type UseRiveOptions = {
    useDevicePixelRatio: boolean;
    customDevicePixelRatio: number;
    fitCanvasToArtboardHeight: boolean;
    /**
     * For `@rive-app/react-webgl2`, share one WebGL context across every canvas
     * on the page instead of giving this canvas its own.
     *
     * **We recommend leaving this unset**, which picks the right value for you:
     * - `true` normally, so multiple Rive graphics on a page share one context.
     * - `false` when `enableGPUCanvas` is on, because GPU Canvas needs a context
     *   of its own for each `<canvas>`.
     *
     * Setting it explicitly always wins. Setting it to `true` alongside
     * `enableGPUCanvas` means GPU Canvas content will not draw.
     */
    useOffscreenRenderer: boolean;
    shouldResizeCanvasToContainer: boolean;
    shouldUseIntersectionObserver?: boolean;
};
/**
 * @typedef RiveState
 * @property canvas - Canvas element the Rive Animation is attached to.
 * @property container - Container element of the canvas.
 * @property setCanvasRef - Ref callback to be passed to the canvas element.
 * @property setContainerRef - Ref callback to be passed to the container
 * element of the canvas. This is optional, however if not used then the hook
 * will not take care of automatically resizing the canvas to it's outer
 *   container if the window resizes.
 * @property rive - The loaded Rive Animation
 */
type RiveState = {
    canvas: HTMLCanvasElement | null;
    container: HTMLElement | null;
    setCanvasRef: RefCallback<HTMLCanvasElement>;
    setContainerRef: RefCallback<HTMLElement>;
    rive: Rive$1 | null;
    RiveComponent: (props: ComponentProps<'canvas'>) => JSX.Element;
};
type UseRiveFileParameters = Partial<Omit<RiveFileParameters, 'onLoad' | 'onLoadError'>>;
type FileStatus = 'idle' | 'loading' | 'failed' | 'success';
/**
 * @typedef RiveFileState
 * @property data - The RiveFile instance
 * @property status - The status of the file
 */
type RiveFileState = {
    riveFile: RiveFile | null;
    status: FileStatus;
};
/**
 * Parameters for useViewModel hook.
 *
 * @property name - When provided, specifies the name of the ViewModel to retrieve.
 * @property useDefault - When true, uses the default ViewModel from the Rive instance.
 */
type UseViewModelParameters = {
    name: string;
    useDefault?: never;
} | {
    useDefault?: boolean;
    name?: never;
};
/**
 * Parameters for useViewModelInstance hook.
 *
 * @property name - When provided, specifies the name of the instance to retrieve.
 * @property useDefault - When true, uses the default instance from the ViewModel.
 * @property useNew - When true, creates a new instance of the ViewModel.
 * @property rive - If provided, automatically binds the instance to this Rive instance.
 */
type UseViewModelInstanceParameters = {
    name: string;
    useDefault?: never;
    useNew?: never;
    rive?: Rive$1 | null;
} | {
    useDefault?: boolean;
    name?: never;
    useNew?: never;
    rive?: Rive$1 | null;
} | {
    useNew?: boolean;
    name?: never;
    useDefault?: never;
    rive?: Rive$1 | null;
};
/**
 * Parameters for useGlobalViewModelInstance. Selects which instance to resolve
 * from the view model (default when none given); `rive` binds it.
 *
 * @property instanceName - Resolve the view model instance with this instance name.
 * @property useNew - Create a new blank instance of the view model.
 * @property instance - Register this caller-supplied instance directly. May be null while it loads.
 * @property rive - Registers the instance as the global and schedules a coalesced
 * bind. Required for the hook to register/bind anything.
 */
type UseGlobalViewModelInstanceParameters = {
    instanceName: string;
    useNew?: never;
    instance?: never;
    rive?: Rive$1 | null;
} | {
    useNew?: boolean;
    instanceName?: never;
    instance?: never;
    rive?: Rive$1 | null;
} | {
    instance?: ViewModelInstance | null;
    instanceName?: never;
    useNew?: never;
    rive?: Rive$1 | null;
};
/**
 * Parameters for interacting with trigger properties of a ViewModelInstance
 * @property onTrigger - Callback that runs when the trigger fires
 */
type UseViewModelInstanceTriggerParameters = {
    onTrigger?: () => void;
};
type UseViewModelInstanceNumberResult = {
    /**
     * The current value of the number.
     */
    value: number | null;
    /**
     * Set the value of the number.
     * @param value - The value to set the number to.
     */
    setValue: (value: number) => void;
};
type UseViewModelInstanceStringResult = {
    /**
     * The current value of the string.
     */
    value: string | null;
    /**
     * Set the value of the string.
     * @param value - The value to set the string to.
     */
    setValue: (value: string) => void;
};
type UseViewModelInstanceBooleanResult = {
    /**
     * The current value of the boolean.
     */
    value: boolean | null;
    /**
     * Set the value of the boolean.
     * @param value - The value to set the boolean to.
     */
    setValue: (value: boolean) => void;
};
type UseViewModelInstanceColorResult = {
    /**
     * The current value of the color.
     */
    value: number | null;
    /**
     * Set the value of the color.
     * @param value - The value to set the color to.
     */
    setValue: (value: number) => void;
    /**
     * Set the red value of the color.
     * @param r - The red value to set the color to.
     */
    setRgb: (r: number, g: number, b: number) => void;
    /**
     * Set the red, green, blue, and alpha values of the color.
     * @param r - The red value to set the color to.
     * @param g - The green value to set the color to.
     * @param b - The blue value to set the color to.
     * @param a - The alpha value to set the color to.
     */
    setRgba: (r: number, g: number, b: number, a: number) => void;
    /**
     * Set the alpha value of the color.
     * @param a - The alpha value to set the color to.
     */
    setAlpha: (a: number) => void;
    /**
     * Set the opacity value of the color.
     * @param o - The opacity value to set the color to.
     */
    setOpacity: (o: number) => void;
};
type UseViewModelInstanceEnumResult = {
    /**
     * The current value of the enum.
     */
    value: string | null;
    /**
     * Set the value of the enum.
     * @param value - The value to set the enum to.
     */
    setValue: (value: string) => void;
    /**
     * The values of the enum.
     */
    values: string[];
};
type UseViewModelInstanceTriggerResult = {
    /**
     * Fires the property trigger.
     */
    trigger: () => void;
};
type RiveRenderImage = Awaited<ReturnType<typeof decodeImage>>;
type UseViewModelInstanceImageResult = {
    /**
     * Set the value of the image.
     * @param value - The image to set.
     */
    setValue: (value: RiveRenderImage | null) => void;
};
type RiveDecodedFont = Awaited<ReturnType<typeof decodeFont>>;
type UseViewModelInstanceFontResult = {
    /**
     * Set the value of the font.
     * @param value - The decoded font to set (from `decodeFont`), or null to clear.
     */
    setValue: (value: RiveDecodedFont | null) => void;
};
type UseViewModelInstanceListResult = {
    /**
     * The current length of the list.
     */
    length: number;
    /**
     * Add an instance to the end of the list.
     * @param instance - The ViewModelInstance to add.
     */
    addInstance: (instance: ViewModelInstance) => void;
    /**
     * Add an instance at a specific index in the list.
     * @param instance - The ViewModelInstance to add.
     * @param index - The index to add the instance at.
     * @returns True if the instance was successfully added, false otherwise.
     */
    addInstanceAt: (instance: ViewModelInstance, index: number) => boolean;
    /**
     * Remove an instance from the list.
     * @param instance - The ViewModelInstance to remove.
     */
    removeInstance: (instance: ViewModelInstance) => void;
    /**
     * Remove an instance at a specific index from the list.
     * @param index - The index to remove the instance from.
     */
    removeInstanceAt: (index: number) => void;
    /**
     * Get an instance at a specific index from the list.
     * @param index - The index to get the instance from.
     * @returns The ViewModelInstance at the index, or null if not found.
     */
    getInstanceAt: (index: number) => ViewModelInstance | null;
    /**
     * Swap two instances in the list.
     * @param a - The first index.
     * @param b - The second index.
     */
    swap: (a: number, b: number) => void;
};
type UseViewModelInstanceArtboardResult = {
    /**
     * Set the value of the artboard.
     * @param value - The artboard to set.
     */
    setValue: (value: ViewModelInstanceArtboard extends {
        value: infer T;
    } ? T : never) => void;
};

/**
 * Custom Hook for loading a Rive file.
 *
 * Waits until the load event has fired before returning it.
 * We can then listen for changes to this animation in other hooks to detect
 * when it has loaded.
 *
 * @param riveParams - Object containing parameters accepted by the Rive object
 *   in the rive-js runtime, with the exception of Canvas as that is attached
 *   via the ref callback `setCanvasRef`.
 *
 * @param opts - Optional list of options that are specific for this hook.
 * @returns {RiveAnimationState}
 */
declare function useRive(riveParams?: UseRiveParameters, opts?: Partial<UseRiveOptions>): RiveState;

/**
 * Custom hook for fetching a stateMachine input from a rive file.
 *
 * @deprecated State machine inputs are deprecated and will be removed in a
 * future major version: please use data binding properties instead. See
 * {@link https://rive.app/docs/editor/data-binding/migration-guide#state-machine-inputs}
 * for how to migrate.
 *
 * @param rive - Rive instance
 * @param stateMachineName - Name of the state machine
 * @param inputName - Name of the input
 * @returns
 */
declare function useStateMachineInput(rive: Rive$1 | null, stateMachineName?: string, inputName?: string, initialValue?: number | boolean): StateMachineInput | null;

/**
 * Hook for fetching a ViewModel from a Rive instance.
 *
 * @param rive - The Rive instance to retrieve the ViewModel from
 * @param params - Options for retrieving a ViewModel
 * @param params.name - When provided, specifies the name of the ViewModel to retrieve
 * @param params.useDefault - When true, uses the default ViewModel from the Rive instance
 * @returns The ViewModel or null if not found
 */
declare function useViewModel(rive: Rive$1 | null, params?: UseViewModelParameters): ViewModel | null;

/**
 * Hook for fetching a ViewModelInstance from a ViewModel.
 *
 * @param viewModel - The ViewModel to get an instance from
 * @param params - Options for retrieving a ViewModelInstance
 * @param params.name - When provided, specifies the name of the instance to retrieve
 * @param params.useDefault - When true, uses the default instance from the ViewModel
 * @param params.useNew - When true, creates a new instance of the ViewModel
 * @param params.rive - If provided, automatically binds the instance to this Rive instance
 * @returns The ViewModelInstance or null if not found
 */
declare function useViewModelInstance(viewModel: ViewModel | null, params?: UseViewModelInstanceParameters): ViewModelInstance | null;

/**
 * Creates and binds a *global* view model instance. Resolves an
 * instance from `viewModel` — default, or `{ useNew }` / `{ instanceName }` /
 * `{ instance }` (a caller-supplied view model instance). When
 * `params.rive` is set, registers it as the global `name` and schedules a
 * coalesced bind (see {@link scheduleBind}).
 *
 * To read an already-bound global view model instance instead, query rive directly
 * (rive.globalViewModelInstance(name)).
 *
 * @param viewModel - ViewModel to resolve from (e.g. from useViewModel). May be
 *   null while it loads, or when supplying `params.instance` directly.
 * @param name - Name of the global view model to register under.
 * @param params - Which instance to resolve, and the Rive instance to register with.
 * @returns The resolved ViewModelInstance, or null if unavailable.
 */
declare function useGlobalViewModelInstance(viewModel: ViewModel | null, name: string, params?: UseGlobalViewModelInstanceParameters): ViewModelInstance | null;

/**
 * Hook for interacting with number properties of a ViewModelInstance.
 *
 * @param params - Parameters for interacting with number properties
 * @param params.path - Path to the number property (e.g. "speed" or "group/speed")
 * @param params.viewModelInstance - The ViewModelInstance containing the number property
 * @returns An object with the number value and a setter function
 */
declare function useViewModelInstanceNumber(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceNumberResult;

/**
 * Hook for interacting with string properties of a ViewModelInstance.
 *
 * @param params - Parameters for interacting with string properties
 * @param params.path - Path to the property (e.g. "text" or "nested/text")
 * @param params.viewModelInstance - The ViewModelInstance containing the string property
 * @returns An object with the string value and a setter function
 */
declare function useViewModelInstanceString(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceStringResult;

/**
 * Hook for interacting with boolean ViewModel instance properties.
 *
 * @param path - The path to the boolean property
 * @param viewModelInstance - The ViewModelInstance containing the boolean property to operate on
 * @returns An object with the boolean value and a setter function
 */
declare function useViewModelInstanceBoolean(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceBooleanResult;

/**
 * Hook for interacting with color properties of a ViewModelInstance.
 *
 * @param path - Path to the color property
 * @param viewModelInstance - The ViewModelInstance containing the color property
 * @returns An object with the color value and setter functions for different color formats
 */
declare function useViewModelInstanceColor(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceColorResult;

/**
 * Hook for interacting with enum properties of a ViewModelInstance.
 *
 * @param params - Parameters for interacting with enum properties
 * @param params.path - Path to the enum property (e.g. "state" or "group/state")
 * @param params.viewModelInstance - The ViewModelInstance containing the enum property
 * @returns An object with the enum value, available values, and a setter function
 */
declare function useViewModelInstanceEnum(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceEnumResult;

/**
 * Hook for interacting with trigger properties of a ViewModelInstance.
 *
 * @param params - Parameters for interacting with trigger properties
 * @param params.path - Path to the trigger property (e.g. "onTap" or "group/onTap")
 * @param params.viewModelInstance - The ViewModelInstance containing the trigger property
 * @param params.onTrigger - Callback that runs when the trigger is fired
 * @returns An object with a trigger function
 */
declare function useViewModelInstanceTrigger(path: string, viewModelInstance?: ViewModelInstance | null, params?: UseViewModelInstanceTriggerParameters): UseViewModelInstanceTriggerResult;

/**
 * Hook for interacting with image properties of a ViewModelInstance.
 *
 * @param path - Path to the image property (e.g. "profileImage" or "group/avatar")
 * @param viewModelInstance - The ViewModelInstance containing the image property
 * @returns An object with a setter function to set a new image value
 */
declare function useViewModelInstanceImage(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceImageResult;

/**
 * Hook for interacting with font properties of a ViewModelInstance.
 *
 * @param path - Path to the font property (e.g. "boundFont" or "group/titleFont")
 * @param viewModelInstance - The ViewModelInstance containing the font property
 * @returns An object with a setter function to set a new font value
 */
declare function useViewModelInstanceFont(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceFontResult;

/**
 * Hook for interacting with list properties of a ViewModelInstance.
 *
 * @param path - Path to the property (e.g. "items" or "nested/items")
 * @param viewModelInstance - The ViewModelInstance containing the list property
 * @returns An object with the list length and manipulation functions
 */
declare function useViewModelInstanceList(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceListResult;

interface UseResizeCanvasProps {
    /**
     * Whether or not Rive is loaded and renderer is associated with the canvas
     */
    riveLoaded: boolean;
    /**
     * Ref to the canvas element
     */
    canvasElem: HTMLCanvasElement | null;
    /**
     * Ref to the container element of the canvas
     */
    containerRef: MutableRefObject<HTMLElement | null>;
    /**
     * (Optional) Callback to be invoked after the canvas has been resized due to a resize
     * of its parent container. This is where you would want to reset the layout
     * dimensions for the Rive renderer to dictate the new min/max bounds of the
     * canvas.
     *
     * Using the high-level JS runtime, this might be a simple call to `rive.resizeToCanvas()`
     * Using the low-level JSruntime, this might be invoking the renderer's `.align()` method
     * with the Layout and min/max X/Y values of the canvas.
     *
     * @returns void
     */
    onCanvasHasResized?: () => void;
    /**
     * (Optional) Options passed to the useRive hook, including the shouldResizeCanvasToContainer option
     * which prevents the canvas element from resizing to its parent container
     */
    options?: Partial<UseRiveOptions>;
    /**
     * (Optional) AABB bounds of the artboard. If provided, the canvas will be sized to the artboard
     * height if the fitCanvasToArtboardHeight option is true.
     */
    artboardBounds?: Bounds;
}
/**
 * Helper hook to listen for changes in the <canvas> parent container size and size the <canvas>
 * to match. If a resize event has occurred, a supplied callback (onCanvasHasResized)
 * will be inokved to allow for any re-calculation needed (i.e. Rive layout on the canvas).
 *
 * This hook is useful if you are not intending to use the `useRive` hook yourself, but still
 * want to use the auto-sizing logic on the canvas/container.
 *
 * @param props - Object to supply necessary props to the hook
 */
declare function useResizeCanvas({ riveLoaded, canvasElem, containerRef, options, onCanvasHasResized, artboardBounds, }: UseResizeCanvasProps): void;

/**
 * Custom hook for initializing and managing a RiveFile instance within a component.
 * It sets up a RiveFile based on provided source parameters (URL or ArrayBuffer) and ensures
 * proper cleanup to avoid memory leaks when the component unmounts or inputs change.
 *
 * @param params - Object containing parameters accepted by the Rive file in the @rive-app/canvas runtime,
 *
 * @returns {RiveFileState} Contains the active RiveFile instance (`riveFile`) and the loading status.
 */
declare function useRiveFile(params: UseRiveFileParameters): RiveFileState;

/**
 * Hook for interacting with artboard properties of a ViewModelInstance.
 *
 * @param path - Path to the artboard property (e.g. "targetArtboard" or "group/artboard")
 * @param viewModelInstance - The ViewModelInstance containing the artboard property
 * @returns An object with a setter function
 */
declare function useViewModelInstanceArtboard(path: string, viewModelInstance?: ViewModelInstance | null): UseViewModelInstanceArtboardResult;

export { RiveProps, RiveState, UseRiveFileParameters, UseRiveOptions, UseRiveParameters, Rive as default, useGlobalViewModelInstance, useResizeCanvas, useRive, useRiveFile, useStateMachineInput, useViewModel, useViewModelInstance, useViewModelInstanceArtboard, useViewModelInstanceBoolean, useViewModelInstanceColor, useViewModelInstanceEnum, useViewModelInstanceFont, useViewModelInstanceImage, useViewModelInstanceList, useViewModelInstanceNumber, useViewModelInstanceString, useViewModelInstanceTrigger };
