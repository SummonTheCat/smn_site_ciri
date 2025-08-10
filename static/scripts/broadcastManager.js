class BroadcastManager {
    /**
     * @param {string|string[]} contextCodes - Code(s) this page cares about
     * @param {function} onMatch - Function to run when a broadcast matches any context code
     */
    constructor(contextCodes, onMatch) {
        this.onMatch = typeof onMatch === 'function' ? onMatch : () => {};
        this.es = null;
        this.contextCodes = new Set();

        this.setContextIDs(contextCodes);
        this._initEventSource();
    }

    _initEventSource() {
        if (this.es) {
            this.es.close();
        }
        this.es = new EventSource('/events');
        this.es.addEventListener('update', e => {
            const id = e.data.trim();
            if (this.contextCodes.has(id)) {
                this.onMatch(id);
            }
        });
        this.es.onopen = () => console.log('[BroadcastManager] Connected to /events');
        this.es.onerror = err => console.warn('[BroadcastManager] Connection error', err);
    }

    /** Set the entire list of context IDs */
    setContextIDs(ids) {
        if (!Array.isArray(ids)) ids = [ids];
        this.contextCodes = new Set(ids.map(String));
        console.log('[BroadcastManager] Context IDs set to', Array.from(this.contextCodes));
    }

    /** Add one context ID */
    addContextID(id) {
        this.contextCodes.add(String(id));
        console.log('[BroadcastManager] Added context ID', id);
    }

    /** Remove one context ID */
    removeContextID(id) {
        this.contextCodes.delete(String(id));
        console.log('[BroadcastManager] Removed context ID', id);
    }

    /** Clear all context IDs */
    clearContextIDs() {
        this.contextCodes.clear();
        console.log('[BroadcastManager] Cleared all context IDs');
    }

    /** Emit to a given ID (fetches /emit/:id) */
    emitToID(id) {
        return fetch(`/emit/${encodeURIComponent(String(id))}`)
            .then(r => r.text())
            .then(t => {
                console.log(`[BroadcastManager] Emit response for ${id}:`, t);
                return t;
            });
    }

    /** Manually close the SSE connection */
    close() {
        if (this.es) {
            this.es.close();
            console.log('[BroadcastManager] Connection closed');
        }
    }
}

window.BroadcastManager = BroadcastManager;
