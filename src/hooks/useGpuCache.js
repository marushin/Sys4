import { useEffect, useState } from 'react';

const KEY_HAS_GPU = 'sys4_has_gpu';
const KEY_CACHED_GPU = 'sys4_cached_gpu';
/** Consecutive samples without GPU data before the GPU section is hidden. */
const MISSES_BEFORE_HIDE = 3;

function readInitial() {
    try {
        const cached = localStorage.getItem(KEY_CACHED_GPU);
        return {
            hasGpu: localStorage.getItem(KEY_HAS_GPU) !== 'false',
            gpu: cached ? JSON.parse(cached) : null,
            misses: 0,
        };
    } catch {
        return { hasGpu: true, gpu: null, misses: 0 };
    }
}

/** Return `value`, or the most recent non-null value seen before it. */
export function useLastDefined(value) {
    const [last, setLast] = useState(value ?? null);
    if (value != null && value !== last) setLast(value);
    return value ?? last;
}

/**
 * Keep the last known GPU sample so the GPU section renders instantly on start-up, and
 * remember whether the machine has a GPU at all (hides the section on GPU-less systems).
 * `resources` must come from a request that asked for `gpu: true`.
 */
export function useGpuCache(resources) {
    const [state, setState] = useState(readInitial);
    const [seen, setSeen] = useState(null);

    // Derive state from each new sample during render (no effect → no extra render pass).
    if (resources && resources !== seen) {
        setSeen(resources);
        setState((s) => {
            if (resources.gpu) return { hasGpu: true, gpu: resources.gpu, misses: 0 };
            const misses = s.misses + 1;
            return { ...s, misses, hasGpu: misses >= MISSES_BEFORE_HIDE ? false : s.hasGpu };
        });
    }

    useEffect(() => {
        try {
            localStorage.setItem(KEY_HAS_GPU, String(state.hasGpu));
            if (state.gpu) localStorage.setItem(KEY_CACHED_GPU, JSON.stringify(state.gpu));
        } catch {
            // ignore
        }
    }, [state.hasGpu, state.gpu]);

    return { hasGpu: state.hasGpu, gpu: resources?.gpu ?? state.gpu };
}
