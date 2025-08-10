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

    /** Public: fetch a component’s HTML (as text) */
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

    /** Public: mount a component into the DOM */
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
          // Capture the first node so we can return a handle to what replaced the target
          const firstNode =
            fragment.firstElementChild || fragment.firstChild || null;
          el.replaceWith(fragment);
          // If there was at least one node, return it; otherwise return the former parent
          return (firstNode && firstNode.nodeType === 1 /* ELEMENT_NODE */)
            ? firstNode
            : (el.parentElement || document.body);
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

    /** Public: fetch and return an embeddable HTML snippet (no DOM changes) */
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
      // Use a <template> so we don't introduce an extra wrapper element.
      const tpl = document.createElement('template');
      tpl.innerHTML = html;
      return tpl.content.cloneNode(true);
    }
  }

  // ---------- Ease-of-use helper ----------

  /**
   * Apply (mount) a component with minimal boilerplate.
   * Creates/reuses a global manager (window.cm), waits for DOM if needed,
   * and by default REPLACES the target element itself (no wrapper).
   *
   * @param {string|Element} targetElement - CSS selector or Element to replace.
   * @param {string} targetComponent - Component name (e.g., "header").
   * @param {any[]|string|number|null|undefined} componentArgs - Args array; non-array becomes [String(value)].
   * @returns {Promise<Element>} resolves with the inserted root element (or parent fallback).
   */
  async function applyComponent(targetElement, targetComponent, componentArgs) {
    // Ensure a singleton manager on window (you can pre-create window.cm yourself too)
    const cm = (window.cm instanceof ComponentManager)
      ? window.cm
      : (window.cm = new ComponentManager({ basePath: '/components', extractBody: true }));

    // Normalize args
    const args = Array.isArray(componentArgs)
      ? componentArgs
      : (componentArgs == null ? [] : [String(componentArgs)]);

    // If target is a selector and DOM isn't ready or element not found yet, wait then retry query once.
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

    // Element case
    if (!(targetElement instanceof Element)) {
      throw new Error('applyComponent: targetElement must be a selector string or an Element.');
    }
    if (document.readyState === 'loading') {
      await new Promise(res => document.addEventListener('DOMContentLoaded', res, { once: true }));
    }
    return cm.mount(targetElement, targetComponent, args, { mode: 'outerReplace' });
  }

  // Expose globally (like TransitionManager)
  window.ComponentManager = ComponentManager;
  window.applyComponent = applyComponent;
})();
