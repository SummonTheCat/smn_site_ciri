
(function () {
  class ComponentManager {
    /**
     * @param {Object} options
     * @param {string}  [options.basePath='/components'] - Base route for components.
     * @param {boolean} [options.extractBody=true]       - If true, extracts <body> innerHTML from full documents.
     * @param {boolean} [options.fadeIn=false]           - If true, fade in newly mounted content.
     * @param {number}  [options.fadeInDuration=250]     - Fade duration in ms.
     * @param {string}  [options.fadeInEasing='ease-out']- CSS easing for fade.
     */
    constructor({
      basePath = '/components',
      extractBody = true,
      fadeIn = false,
      fadeInDuration = 250,
      fadeInEasing = 'ease-out',
    } = {}) {
      if (typeof window === 'undefined' || typeof window.fetch !== 'function') {
        throw new Error('ComponentManager: must run in a browser environment with fetch support.');
      }
      this.basePath = (basePath || '/components').replace(/\/+$/, '');
      this.extractBody = !!extractBody;

      // Fade options
      this.fadeIn = !!fadeIn;
      this.fadeInDuration = Math.max(0, Number(fadeInDuration) || 0);
      this.fadeInEasing = String(fadeInEasing || 'ease-out');
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
          // Replace the inner HTML of target; fade the container itself for a smooth swap.
          if (this.fadeIn) this._prepareFade(el);
          el.innerHTML = html;
          if (this.fadeIn) this._runFade([el]);
          return el;
        }

        case 'outerReplace': {
          const fragment = this._htmlToFragment(html);
          const inserted = this._nodesOfFragment(fragment);
          el.replaceWith(fragment);
          if (this.fadeIn) this._runFade(inserted);
          const firstNode =
            inserted.find(n => n.nodeType === 1) || null;
          return firstNode || (el.parentElement || document.body);
        }

        case 'append': {
          const fragment = this._htmlToFragment(html);
          const inserted = this._nodesOfFragment(fragment);
          el.appendChild(fragment);
          if (this.fadeIn) this._runFade(inserted);
          return el;
        }

        case 'prepend': {
          const fragment = this._htmlToFragment(html);
          const inserted = this._nodesOfFragment(fragment);
          el.insertBefore(fragment, el.firstChild);
          if (this.fadeIn) this._runFade(inserted);
          return el;
        }

        case 'before': {
          const fragment = this._htmlToFragment(html);
          const inserted = this._nodesOfFragment(fragment);
          el.parentNode ? el.parentNode.insertBefore(fragment, el) : null;
          if (this.fadeIn) this._runFade(inserted);
          return el.previousElementSibling || el;
        }

        case 'after': {
          const fragment = this._htmlToFragment(html);
          const inserted = this._nodesOfFragment(fragment);
          el.parentNode
            ? el.parentNode.insertBefore(fragment, el.nextSibling)
            : null;
          if (this.fadeIn) this._runFade(inserted);
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

    _nodesOfFragment(fragment) {
      // Collect top-level nodes that will be inserted
      const nodes = [];
      for (let n = fragment.firstChild; n; n = n.nextSibling) {
        nodes.push(n);
      }
      return nodes;
    }

    // ---------- fade helpers ----------

    _prepareFade(node) {
      if (!(node instanceof Element)) return;
      node.style.opacity = '0';
      node.style.willChange = 'opacity';
      node.style.transition = `opacity ${this.fadeInDuration}ms ${this.fadeInEasing}`;
    }

    _runFade(nodes) {
      // Apply fade to element nodes only
      const elems = nodes.filter(n => n && n.nodeType === 1);
      if (elems.length === 0) return;

      // Prepare all
      elems.forEach(el => this._prepareFade(el));

      // Kick transitions on next frame
      requestAnimationFrame(() => {
        elems.forEach(el => (el.style.opacity = '1'));
        // Cleanup styles after transition ends
        const cleanup = (el) => {
          const done = () => {
            el.style.willChange = '';
            el.style.transition = '';
            el.removeEventListener('transitionend', done);
          };
          el.addEventListener('transitionend', done);
          // In case transitionend doesn’t fire (display changes, etc.)
          setTimeout(done, this.fadeInDuration + 50);
        };
        elems.forEach(cleanup);
      });
    }
  }

  // ---------- Global helpers (singleton manager) ----------

  async function applyComponent(targetElement, targetComponent, componentArgs) {
    const cm = (window.cm instanceof ComponentManager)
      ? window.cm
      : (window.cm = new ComponentManager({
          basePath: '/components',
          extractBody: true,
          // Set your site-wide defaults here:
          fadeIn: true,
          fadeInDuration: 250,
          fadeInEasing: 'ease-out',
        }));

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
      : (window.cm = new ComponentManager({
          basePath: '/components',
          extractBody: true,
          fadeIn: true,
          fadeInDuration: 250,
          fadeInEasing: 'ease-out',
        }));

    if (document.readyState === 'loading' && batch.some(it => typeof it?.target === 'string')) {
      await new Promise(res => document.addEventListener('DOMContentLoaded', res, { once: true }));
    }
    return cm.mountGroup(batch, opts);
  }

  window.ComponentManager = ComponentManager;
  window.applyComponent = applyComponent;
  window.applyComponentsGroup = applyComponentsGroup;
})();