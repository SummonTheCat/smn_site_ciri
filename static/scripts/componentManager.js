
(function () {
  class ComponentManager {
    /**
     * @param {Object} options
     * @param {string} [options.basePath='/components'] - Base route for components.
     * @param {boolean} [options.extractBody=true] - If true, extracts <body> innerHTML from full documents.
     */
    constructor({
      basePath = '/components',
      extractBody = true,
    } = {}) {
      if (typeof window === 'undefined' || typeof window.fetch !== 'function') {
        throw new Error('ComponentManager: must run in a browser environment with fetch support.');
      }
      this.basePath = (basePath || '/components').replace(/\/+$/, '');
      this.extractBody = !!extractBody;
    }

    /** Fetch a component’s HTML (as text) */
    async fetchComponent(name, compArgs = [], init) {
      if (!name) throw new Error('ComponentManager.fetchComponent: name is required.');
      const url = `${this.basePath}/${encodeURIComponent(name)}`;

      const res = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...(init && init.headers) },
        body: JSON.stringify({ compArgs }),
        ...(init || {}),
      });

      const text = await res.text().catch(() => '');
      if (!res.ok) {
        throw new Error(`Component "${name}" failed: ${res.status} ${res.statusText}\n${text}`);
      }
      return text;
    }

    /** Mount a component into the DOM */
    async mount(target, name, compArgs = [], { mode = 'replace' } = {}) {
      const el = this._resolveTarget(target);
      const html = this._toEmbeddable(await this.fetchComponent(name, compArgs));

      switch (mode) {
        case 'replace': {
          el.innerHTML = html;
          return el;
        }
        case 'outerReplace': {
          const fragment = this._htmlToFragment(html);
          const firstNode = fragment.firstElementChild || fragment.firstChild || null;
          el.replaceWith(fragment);
          return (firstNode && firstNode.nodeType === 1) ? firstNode : (el.parentElement || document.body);
        }
        case 'append': {
          el.insertAdjacentHTML('beforeend', html);
          return el;
        }
        case 'prepend': {
          el.insertAdjacentHTML('afterbegin', html);
          return el;
        }
        case 'before': {
          el.insertAdjacentHTML('beforebegin', html);
          return el.previousElementSibling || el;
        }
        case 'after': {
          el.insertAdjacentHTML('afterend', html);
          return el.nextElementSibling || el;
        }
        default:
          throw new Error(`ComponentManager.mount: unknown mode "${mode}"`);
      }
    }

    /** Batch: mount multiple components (in parallel). */
    async mountGroup(
      batch,
      {
        mode = 'outerReplace',
        onProgress = null, // (index, total, resultOrError)
      } = {}
    ) {
      if (!Array.isArray(batch) || batch.length === 0) return [];
      const total = batch.length;

      // Kick off all fetches first for better parallelism
      const jobs = batch.map(async (item, i) => {
        const { target, name, args = [], mountMode = mode } = item || {};
        try {
          const res = await this.mount(target, name, args, { mode: mountMode });
          onProgress && onProgress(i + 1, total, { ok: true, target, name, res });
          return res;
        } catch (err) {
          onProgress && onProgress(i + 1, total, { ok: false, target, name, error: err });
          throw err;
        }
      });

      // Fail-fast if any component throws
      return Promise.all(jobs);
    }

    /** Fetch and return an embeddable HTML snippet (no DOM changes) */
    async render(name, compArgs = [], init) {
      return this._toEmbeddable(await this.fetchComponent(name, compArgs, init));
    }

    // ---------- internals ----------
    _resolveTarget(target) {
      if (target instanceof Element) return target;
      if (typeof target === 'string') {
        const el = document.querySelector(target);
        if (el) return el;
        throw new Error(`ComponentManager: target selector not found: ${target}`);
      }
      throw new Error('ComponentManager: invalid target, pass a selector string or Element.');
    }

    _toEmbeddable(html) {
      if (!this.extractBody) return html;
      if (!/<\s*html[\s>]|<!doctype/i.test(html)) return html;
      try {
        const parser = new DOMParser();
        const doc = parser.parseFromString(html, 'text/html');
        return doc.body ? doc.body.innerHTML : html;
      } catch {
        return html;
      }
    }

    _htmlToFragment(html) {
      const tpl = document.createElement('template');
      tpl.innerHTML = html;
      return tpl.content.cloneNode(true);
    }
  }

  // ---------- Global helpers (singleton manager) ----------

  async function applyComponent(targetElement, targetComponent, componentArgs) {
    const cm = (window.cm instanceof ComponentManager)
      ? window.cm
      : (window.cm = new ComponentManager({ basePath: '/components', extractBody: true }));

    const args = Array.isArray(componentArgs)
      ? componentArgs
      : (componentArgs == null ? [] : [String(componentArgs)]);

    if (typeof targetElement === 'string') {
      const selector = targetElement;
      let el = document.querySelector(selector);
      if (!el && document.readyState === 'loading') {
        await new Promise(res => document.addEventListener('DOMContentLoaded', res, { once: true }));
        el = document.querySelector(selector);
      }
      if (!el) throw new Error(`applyComponent: target selector not found: ${selector}`);
      return cm.mount(el, targetComponent, args, { mode: 'outerReplace' });
    }

    if (!(targetElement instanceof Element)) {
      throw new Error('applyComponent: targetElement must be a selector string or an Element.');
    }
    if (document.readyState === 'loading') {
      await new Promise(res => document.addEventListener('DOMContentLoaded', res, { once: true }));
    }
    return cm.mount(targetElement, targetComponent, args, { mode: 'outerReplace' });
  }

  /**
   * Apply a batch of components (parallel), returns when all are mounted.
   * @param {Array<{target:string|Element,name:string,args?:any[],mountMode?:string}>} batch
   * @param {{mode?:string,onProgress?:(i:number,total:number,res:any)=>void}} opts
   */
  async function applyComponentsGroup(batch, opts = {}) {
    const cm = (window.cm instanceof ComponentManager)
      ? window.cm
      : (window.cm = new ComponentManager({ basePath: '/components', extractBody: true }));
    // Wait for DOM if any string selector appears and DOM is still loading
    if (document.readyState === 'loading' && batch.some(it => typeof it?.target === 'string')) {
      await new Promise(res => document.addEventListener('DOMContentLoaded', res, { once: true }));
    }
    return cm.mountGroup(batch, opts);
  }

  window.ComponentManager = ComponentManager;
  window.applyComponent = applyComponent;
  window.applyComponentsGroup = applyComponentsGroup;
})();